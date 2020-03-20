//! All values in NP_Buffers are accessed and modified through NP_Ptrs
//! 
//! NP_ Pointers are the primary abstraction to read, update or delete values in a buffer.
//! Pointers should *never* be created directly, instead the various methods provided by the library to access
//! the internals of the buffer should be used.
//! 
//! Once you have a pointer you can read it's contents if it's a scalar value (`.to_string()`, `.to_int8()`, etc) or convert it to a collection type (`.as_table()`, `.as_map()`, etc).
//! When you attempt to read, update, or convert a pointer the schema is checked for that pointer location.  If the schema conflicts with what you're attempting to do, for example
//! if you call `to_string()` but the schema for that location is of `int8` type, the operation will fail.  As a result, you should be careful to make sure your reads and updates to the 
//! buffer line up with the schema you provided.

pub mod misc;
pub mod string;
pub mod bytes;
pub mod any;
pub mod numbers;

use crate::json_flex::JFObject;
use crate::memory::NP_Memory;
use crate::NP_Error;
use crate::{schema::{NP_Schema}};

use alloc::string::String;
use alloc::boxed::Box;
use alloc::borrow::ToOwned;

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub enum NP_PtrKinds {
    None,
    // scalar / collection
    Standard  { value: u32 }, // 4 bytes [4]

    // collection items
    MapItem   { value: u32, next: u32, key: u32 },  // 12 bytes [4, 4, 4]
    TableItem { value: u32, next: u32, i: u8    },  // 9  bytes [4, 4, 1]
    ListItem  { value: u32, next: u32, i: u16   },  // 10 bytes [4, 4, 2]
}

impl NP_PtrKinds {
    pub fn get_value(&self) -> u32 {
        match self {
            NP_PtrKinds::None                                                => { 0 },
            NP_PtrKinds::Standard  { value } =>                      { *value },
            NP_PtrKinds::MapItem   { value, key: _,  next: _ } =>    { *value },
            NP_PtrKinds::TableItem { value, i: _,    next: _ } =>    { *value },
            NP_PtrKinds::ListItem  { value, i:_ ,    next: _ } =>    { *value }
        }
    }
}

pub trait NP_Value {
    fn new<T: NP_Value + Default>() -> Self;
    fn is_type(_type_str: &str) -> bool { false }
    fn type_idx() -> (i64, String) { (-1, "null".to_owned()) }
    fn self_type_idx(&self) -> (i64, String) { (-1, "null".to_owned()) }
    fn schema_state(_type_string: &str, _json_schema: &JFObject) -> core::result::Result<i64, NP_Error> { Ok(0) }
    fn buffer_get(_address: u32, _kind: &NP_PtrKinds, _schema: &NP_Schema, _buffer: &NP_Memory) -> core::result::Result<Option<Box<Self>>, NP_Error> {
        let mut message = "This type (".to_owned();
        message.push_str(Self::type_idx().1.as_str());
        message.push_str(") doesn't support .get()!");
        Err(NP_Error::new(message.as_str()))
    }
    fn buffer_set(_address: u32, _kind: &NP_PtrKinds, _schema: &NP_Schema, _buffer: &NP_Memory, _value: Box<&Self>) -> core::result::Result<NP_PtrKinds, NP_Error> {
        let mut message = "This type (".to_owned();
        message.push_str(Self::type_idx().1.as_str());
        message.push_str(") doesn't support .set()!");
        Err(NP_Error::new(message.as_str()))
    }
}

pub trait NP_ValueInto<'a> {
    fn buffer_into(_address: u32, _kind: NP_PtrKinds, _schema: &'a NP_Schema, _buffer: &'a NP_Memory) -> core::result::Result<Option<Box<Self>>, NP_Error> {
        let message = "This type  doesn't support into!".to_owned();
        Err(NP_Error::new(message.as_str()))
    }
}


/// The base data type, all information is stored/retrieved against pointers
/// 
/// Each pointer represents at least a 32 bit unsigned integer that is either zero for no value or points to an offset in the buffer.  All pointer addresses are zero based against the beginning of the buffer.

pub struct NP_Ptr<'a, T: NP_Value + Default + NP_ValueInto<'a>> {
    address: u32, // pointer location
    kind: NP_PtrKinds,
    pub memory: &'a NP_Memory,
    pub schema: &'a NP_Schema,
    pub value: T
}

impl<'a, T: NP_Value + Default + NP_ValueInto<'a>> NP_Ptr<'a, T> {

    pub fn get(&mut self) -> core::result::Result<Option<T>, NP_Error> {


        let value = T::buffer_get(self.address, &self.kind, self.schema, &self.memory)?;
        
        Ok(match value {
            Some (x) => {
                Some(*x)
            },
            None => None
        })
    }

    pub fn set(&mut self, value: T) -> core::result::Result<(), NP_Error> {
        self.kind = T::buffer_set(self.address, &self.kind, self.schema, &self.memory, Box::new(&value))?;
        Ok(())
    }

    #[doc(hidden)]
    pub fn new_standard_ptr(address: u32, schema: &'a NP_Schema, memory: &'a NP_Memory) -> Self {

        let addr = address as usize;
        let value: [u8; 4] = *memory.get_4_bytes(addr).unwrap_or(&[0; 4]);
        
        NP_Ptr {
            address: address,
            kind: NP_PtrKinds::Standard { value: u32::from_le_bytes(value) },
            memory: memory,
            schema: schema,
            value: T::default()
        }
    }

    #[doc(hidden)]
    pub fn new_table_item_ptr(address: u32, schema: &'a NP_Schema, memory: &'a NP_Memory) -> Self {

        let addr = address as usize;
        let b_bytes = &memory.read_bytes();

        let value: [u8; 4] = *memory.get_4_bytes(addr).unwrap_or(&[0; 4]);
        let next: [u8; 4] = *memory.get_4_bytes(addr + 4).unwrap_or(&[0; 4]);
        let index: u8 = b_bytes[addr + 8];

        NP_Ptr {
            address: address,
            kind: NP_PtrKinds::TableItem { 
                value: u32::from_le_bytes(value),
                next: u32::from_le_bytes(next),
                i: index
            },
            memory: memory,
            schema: schema,
            value: T::default()
        }
    }

    #[doc(hidden)]
    pub fn new_map_item_ptr(address: u32, schema: &'a NP_Schema, memory: &'a NP_Memory) -> Self {

        let addr = address as usize;
        let value: [u8; 4] = *memory.get_4_bytes(addr).unwrap_or(&[0; 4]);
        let next: [u8; 4] = *memory.get_4_bytes(addr + 4).unwrap_or(&[0; 4]);
        let key: [u8; 4] = *memory.get_4_bytes(addr + 8).unwrap_or(&[0; 4]);

        NP_Ptr {
            address: address,
            kind: NP_PtrKinds::MapItem { 
                value: u32::from_le_bytes(value),
                next: u32::from_le_bytes(next),
                key: u32::from_le_bytes(key)
            },
            memory: memory,
            schema: schema,
            value: T::default()
        }
    }

    #[doc(hidden)]
    pub fn new_list_item_ptr(address: u32, schema: &'a NP_Schema, memory: &'a NP_Memory) -> Self {

        let addr = address as usize;
        let value: [u8; 4] = *memory.get_4_bytes(addr).unwrap_or(&[0; 4]);
        let next: [u8; 4] = *memory.get_4_bytes(addr + 4).unwrap_or(&[0; 4]);
        let index: [u8; 2] = *memory.get_2_bytes(addr + 8).unwrap_or(&[0; 2]);

        NP_Ptr {
            address: address,
            kind: NP_PtrKinds::ListItem { 
                value: u32::from_le_bytes(value),
                next: u32::from_le_bytes(next),
                i: u16::from_le_bytes(index)
            },
            memory: memory,
            schema: schema,
            value: T::default()
        }
    }

    pub fn has_value(&mut self) -> bool {
        if self.address == 0 { return false; } else { return true; }
    }

    pub fn clear(self) -> core::result::Result<NP_Ptr<'a, T>, NP_Error> {
        Ok(NP_Ptr {
            address: self.address,
            kind: self.memory.set_value_address(self.address, 0, &self.kind),
            memory: self.memory,
            schema: self.schema,
            value: self.value
        })
    }

    pub fn into(self) -> core::result::Result<Option<T>, NP_Error> {
        let result = T::buffer_into(self.address, self.kind, self.schema, &self.memory)?;

        Ok(match result {
            Some(x) => Some(*x),
            None => None
        })
    }

    /*
    pub fn as_table(&mut self) -> core::result::Result<T, NP_Error> {

        match &*self.schema.kind {
            NP_SchemaKinds::Table { columns } => {

                let mut addr = self.kind.get_value();

                let mut head: [u8; 4] = [0; 4];

                // no table here, make one
                if addr == 0 {
                    // no table here, make one
                    let mut memory = self.memory.try_borrow_mut()?;
                    addr = memory.malloc([0 as u8; 4].to_vec())?; // stores HEAD for table
                    memory.set_value_address(self.address, addr, &self.kind)?;
                } else {
                    // existing head, read value
                    let b_bytes = &self.memory.try_borrow()?.bytes;
                    let a = addr as usize;
                    head.copy_from_slice(&b_bytes[a..(a+4)]);
                }

                unsafe { Ok(NP_Table::new(addr, u32::from_le_bytes(head), Rc::clone(&self.memory), &columns)) }
            },
            _ => {
                Err(NP_Error::new(""))
            }
        }
    }*/

/*
    pub fn as_list(&mut self) -> core::result::Result<NP_List, NP_Error> {
        let model = self.schema;

        match &*model.kind {
            NP_SchemaKinds::List { of } => {
                let mut addr = self.get_value_address();
                let mut set_addr = false;

                let mut head: [u8; 4] = [0; 4];
                let mut tail: [u8; 4] = [0; 4];

                // no list here, make one
                if addr == 0 {
                    let mut memory = self.memory.try_borrow_mut()?;

                    addr = memory.malloc([0 as u8; 8].to_vec())?; // stores HEAD & TAIL for list
                    set_addr = true;
                }

                if set_addr { // new head, empty value
                    self.set_value_address(addr)?;
                } else { // existing head, read values
                    let b_bytes = &self.memory.try_borrow()?.bytes;
                    let a = addr as usize;
                    head.copy_from_slice(&b_bytes[a..(a+4)]);
                    tail.copy_from_slice(&b_bytes[(a+4)..(a+8)]);
                }

                Ok(NP_List::new(addr, u32::from_le_bytes(head), u32::from_le_bytes(tail), Rc::clone(&self.memory), &of))
            }
            _ => {
                Err(type_error(TypeReq::Collection, "list", &model))
            }
        }
    }

    pub fn as_tuple(&mut self) -> core::result::Result<NP_Tuple, NP_Error> {

        let model = self.schema;

        match &*model.kind {
            NP_SchemaKinds::Tuple { values } => {
                let mut addr = self.get_value_address();
                let mut set_addr = false;

                let mut head: [u8; 4] = [0; 4];

                // no tuple here, make one
                if addr == 0 {
                    let mut memory = self.memory.try_borrow_mut()?;

                    let value_num = values.len();

                    let mut value_bytes: Vec<u8> = Vec::new();

                    // there is one u32 address for each value
                    for _x in 0..(value_num * 4) {
                        value_bytes.push(0);
                    }

                    addr = memory.malloc(value_bytes)?; // stores HEAD for tuple
                    set_addr = true;
                }

                if set_addr { // new head, empty value
                    self.set_value_address(addr)?;
                } else { // existing head, read value
                    let b_bytes = &self.memory.try_borrow()?.bytes;
                    let a = addr as usize;
                    head.copy_from_slice(&b_bytes[a..(a+4)]);
                }

                Ok(NP_Tuple::new(addr, u32::from_le_bytes(head), Rc::clone(&self.memory), &values))
            }
            _ => {
                Err(type_error(TypeReq::Collection, "tuple", &model))
            }
        }
    }


    pub fn as_map(&mut self) -> core::result::Result<NP_Map, NP_Error> {
        let model = self.schema;

        match &*model.kind {
            NP_SchemaKinds::Map { value } => {

                let mut addr = self.get_value_address();
                let mut set_addr = false;

                let mut head: [u8; 4] = [0; 4];

                // no map here, make one
                if addr == 0 {
                    let mut memory = self.memory.try_borrow_mut()?;

                    addr = memory.malloc([0 as u8; 4].to_vec())?; // stores HEAD for map
                    set_addr = true;
                }

                if set_addr { // new head, empty value
                    self.set_value_address(addr)?;
                } else { // existing head, read value
                    let b_bytes = &self.memory.try_borrow()?.bytes;
                    let a = addr as usize;
                    head.copy_from_slice(&b_bytes[a..(a+4)]);
                }

                Ok(NP_Map::new(addr, u32::from_le_bytes(head), Rc::clone(&self.memory), value))
            }
            _ => {
                Err(type_error(TypeReq::Collection, "map", &model))
            }
        }
    }
 
*/

}


/*
// unsigned integer size:        0 to (2^i) -1
//   signed integer size: -2^(i-1) to  2^(i-1) 
pub enum NP_DataType {
    none,
    /*table {
        head: u32
    },
    map {
        head: u32
    },
    list {
        head: u32,
        tail: u32,
        size: u16
    },
    tuple {
        head: u32
    },*/
    utf8_string { size: u32, value: String },
    bytes { size: u32, value: Vec<u8> },
    int8 { value: i8 },
    int16 { value: i16 },
    int32 { value: i32 },
    int64 { value: i64 }, 
    uint8 { value: u8 },
    uint16 { value: u16 },
    uint32 { value: u32 },
    uint64 { value: u64 },
    float { value: f32 }, // -3.4E+38 to +3.4E+38
    double { value: f64 }, // -1.7E+308 to +1.7E+308
    option { value: u8 }, // enum
    dec32 { value: i32, expo: i8},
    dec64 { value: i64, exp: i8},
    boolean { value: bool },
    geo_16 { lat: f64, lon: f64 }, // (3.5nm resolution): two 64 bit float (16 bytes)
    geo_8 { lat: i32, lon: i32 }, // (16mm resolution): two 32 bit integers (8 bytes) Deg*10000000
    geo_4 { lat: i16, lon: i16 }, // (1.5km resolution): two 16 bit integers (4 bytes) Deg*100
    uuid { value: String }, // 16 bytes 21,267,647,932,558,653,966,460,912,964,485,513,216 possibilities (255^15 * 16) or over 2 quadrillion times more possibilites than stars in the universe
    time_id { id: String, time: u64 }, // 8 + 8 bytes
    date { value: u64 } // 8 bytes  
}*/

// Pointer -> String
/*impl From<&NP_Ptr> for core::result::Result<String> {
    fn from(ptr: &NP_Ptr) -> core::result::Result<String> {
        ptr.to_string()
    }
}*/

/*
// cast i64 => Pointer
impl From<i64> for NP_Value {
    fn from(num: i64) -> Self {
        NP_Value {
            loaded: false,
            address: 0,
            value: NP_Value::int64 { value: num },
            // model: None
        }
    }
}

// cast Pointer => core::result::Result<i64>
impl From<&NP_Value> for core::result::Result<i64> {
    fn from(ptr: &NP_Value) -> core::result::Result<i64> {
        match ptr.value {
            NP_Value::int64 { value } => {
                Some(value)
            }
            _ => None
        }
    }
}*/