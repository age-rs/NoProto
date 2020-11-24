//! All values in buffers are accessed and modified through pointers
//! 
//! NP_Ptr are the primary abstraction to read, update or delete values in a buffer.
//! Pointers should *never* be created directly, instead the various methods provided by the library to access
//! the internals of the buffer should be used.
//! 
//! Once you have a pointer you can read it's contents if it's a scalar value with `.get()` or convert it to a collection with `.deref()`.
//! When you attempt to read, update, or convert a pointer the schema is checked for that pointer location.  If the schema conflicts with the operation you're attempting it will fail.
//! As a result, you should be careful to make sure your reads and updates to the buffer line up with the schema you provided.
//! 
//! 

/// Any type
pub mod any;
pub mod string;
pub mod bytes;
pub mod numbers;
pub mod bool;
pub mod geo;
pub mod dec;
pub mod ulid;
pub mod uuid;
pub mod option;
pub mod date;

use crate::{collection::NP_Collection, pointer::dec::NP_Dec};
use crate::NP_Parsed_Schema;
use crate::{json_flex::NP_JSON};
use crate::memory::{NP_Memory};
use crate::NP_Error;
use crate::{schema::{NP_TypeKeys}, collection::{map::NP_Map, table::NP_Table, list::NP_List, tuple::NP_Tuple}, utils::{print_path}};

use alloc::{boxed::Box, string::String, vec::Vec, borrow::ToOwned};
use bytes::NP_Bytes;

use self::{date::NP_Date, geo::NP_Geo, option::NP_Option, ulid::NP_ULID, uuid::NP_UUID};

#[derive(Debug, Clone, Copy)]
pub struct NP_Cursor_Addr {
    pub address: usize,
    pub is_virtual: bool
}

#[derive(Debug, Clone, Copy)]
pub struct NP_Cursor<'cursor> {
    pub address: usize,
    pub address_value: usize,
    pub schema: &'cursor Box<NP_Parsed_Schema<'cursor>>,
    pub parent_addr: Option<usize>,
    pub kind: NP_Cursor_Kinds,

    pub item_next_addr: Option<usize>,
    pub item_prev_addr: Option<usize>,
    pub item_index: Option<usize>,
    pub item_key_addr: Option<usize>,
    pub item_key: Option<&'cursor str>,

    pub coll_head: Option<usize>,
    pub coll_tail: Option<usize>,
    pub coll_length: Option<usize>
}

impl<'cursor> Default for NP_Cursor<'cursor> {
    fn default() -> Self {
        NP_Cursor {
            address: 0,
            address_value: 0,
            schema: &Box::new(NP_Parsed_Schema::None),
            parent_addr: None,
            kind: NP_Cursor_Kinds::None,
            item_next_addr: None,
            item_prev_addr: None,
            item_index: None,
            item_key_addr: None,
            item_key: None,
            coll_head: None,
            coll_tail: None,
            coll_length: None
        }
    }
}

impl<'cursor> NP_Cursor<'cursor> {


    pub fn new(address: usize, parent: Option<usize>, memory: &NP_Memory, schema: &'cursor Box<NP_Parsed_Schema>) -> Self {
        NP_Cursor {
            address: address,
            address_value: memory.read_address(address),
            schema: &schema,
            parent_addr: parent,
            kind: NP_Cursor_Kinds::Standard,
            item_next_addr: None,
            item_prev_addr: None,
            item_index: None,
            item_key_addr: None,
            item_key: None,
            coll_head: None,
            coll_tail: None,
            coll_length: None
        }
    }

    pub fn get_type_data<'t_data>(cursor_addr: &'t_data NP_Cursor_Addr, memory: &'t_data NP_Memory) -> &'t_data Box<NP_Parsed_Schema<'t_data>> {
        memory.get_cursor_data(&cursor_addr).unwrap().schema
    }


    /// Deep get a value
    /// 
    pub fn select<'sel>(cursor_addr: NP_Cursor_Addr, memory: &'sel NP_Memory<'sel>, path: &[&str], path_index: usize) -> Result<Option<NP_Cursor_Addr>, NP_Error> {

        if path.len() == path_index {
            return Ok(Some(cursor_addr));
        }

        match NP_Cursor::get_type_data(&cursor_addr, &memory).get_type_key() {
            NP_TypeKeys::Table => {
                let new_cursor = NP_Table::select_to_ptr(cursor_addr, memory, &path[path_index], None)?;
                NP_Cursor::select(new_cursor, memory, path, path_index + 1)
            },
            NP_TypeKeys::Map => {
                let new_cursor = NP_Map::select_to_ptr(cursor_addr.address, memory, &path[path_index], false)?;
                NP_Cursor::select(new_cursor, memory, path, path_index + 1)
            },
            NP_TypeKeys::List => {
            
                let list_key = &path[path_index];
                let list_key_int = list_key.parse::<u16>();
                match list_key_int {
                    Ok(x) => {
                        let new_cursor = NP_List::select_to_ptr(cursor_addr, memory, x)?;
                        NP_Cursor::select(new_cursor, memory, path, path_index + 1)
                    },
                    Err(_e) => {
                        let mut err = String::from("Can't query list with string, need number! Path: \n");
                        err.push_str(print_path(&path, path_index).as_str());
                        Err(NP_Error::new(err))
                    }
                }
           
            },
            NP_TypeKeys::Tuple => {

                let list_key = &path[path_index];
                let list_key_int = list_key.parse::<u8>();
                match list_key_int {
                    Ok(x) => {
                        let new_cursor = NP_Tuple::select_to_ptr(cursor_addr, memory, x)?;
                        NP_Cursor::select(new_cursor, memory, path, path_index + 1)
                    },
                    Err(_e) => {
                        let mut err = String::from("Can't query tuple with string, need number! Path: \n");
                        err.push_str(print_path(&path, path_index).as_str());
                        Err(NP_Error::new(err))
                    }
                }
                 
            },
            _ => { 
                // we're not at the end of the select path but we've reached a scalar value
                // so the select has failed to find anything
                return Ok(Some(cursor_addr));
            }
        }
    }

    pub fn select_with_commit<'sel>(cursor_addr: NP_Cursor_Addr, memory: &'sel NP_Memory<'sel>, path: &[&str], path_index: usize) -> Result<Option<NP_Cursor_Addr>, NP_Error> {

        if path.len() == path_index {
            return Ok(Some(cursor_addr));
        }

        match NP_Cursor::get_type_data(&cursor_addr, &memory).get_type_key() {
            NP_TypeKeys::Table => {
                let mut new_cursor = NP_Table::select_to_ptr(cursor_addr, memory, &path[path_index], None)?;
                new_cursor = NP_Table::commit_pointer(&new_cursor, memory)?;
                NP_Cursor::select_with_commit(new_cursor, memory, path, path_index + 1)
            },
            NP_TypeKeys::Map => {

                let mut new_cursor = NP_Map::select_to_ptr(cursor_addr.address, memory, &path[path_index], false)?;
                new_cursor = NP_Map::commit_pointer(&new_cursor, memory)?;
                NP_Cursor::select_with_commit(new_cursor, memory, path, path_index + 1)
            },
            NP_TypeKeys::List => {

                let list_key_int = (&path[path_index]).parse::<u16>();
                match list_key_int {
                    Ok(x) => {
                        let new_cursor = NP_List::select_to_ptr(cursor_addr, memory, x)?;
                        let new_cursor = NP_List::commit_pointer(&new_cursor, memory)?;
                        NP_Cursor::select_with_commit(new_cursor, memory, path, path_index + 1)

                    },
                    Err(_e) => {
                        let mut err = String::from("Can't query list with string, need number! Path: \n");
                        err.push_str(print_path(&path, path_index).as_str());
                        Err(NP_Error::new(err))
                    }
                }
            },
            NP_TypeKeys::Tuple => {

                let list_key = &path[path_index];
                let list_key_int = list_key.parse::<u8>();
                match list_key_int {
                    Ok(x) => {
                        let new_cursor = NP_Tuple::select_to_ptr(cursor_addr, memory, x)?;
                        NP_Cursor::select_with_commit(new_cursor, memory, path, path_index + 1)
                    },
                    Err(_e) => {
                        let mut err = String::from("Can't query tuple with string, need number! Path: \n");
                        err.push_str(print_path(&path, path_index).as_str());
                        Err(NP_Error::new(err))
                    }
                }

            },
            _ => { // scalar type
                
                Ok(Some(cursor_addr))
            }
        }
    }

    /// Get value at this address
    pub fn get_here<'get, T>(cursor_addr: NP_Cursor_Addr, memory: &'get NP_Memory<'get>) -> Result<Option<Box<T>>, NP_Error> where T: Default + NP_Value<'get> {
        if NP_Cursor::get_type_data(&cursor_addr, &memory).into_type_data().0 != T::type_idx().0 {
            return Err(NP_Error::new("typecast error!"))
        }
        match T::into_value(cursor_addr, memory)? {
            Some(x) => {
                Ok(Some(x))
            },
            None => {
                let schema = memory.get_cursor_data(&cursor_addr).unwrap().schema;
                Ok(T::schema_default(schema))
            }
        }
    }

    /// Sets the value at this pointer, only works for scalar types (not collection types).
    /// 
    pub fn set_here<T>(cursor_addr: NP_Cursor_Addr, memory: &NP_Memory, value: T) -> Result<NP_Cursor_Addr, NP_Error> where T: Default + NP_Value<'cursor> {
        if NP_Cursor::get_type_data(&cursor_addr, &memory).into_type_data().0 != T::type_idx().0 {
            return Err(NP_Error::new("typecast error!"))
        }
        T::set_value(cursor_addr, memory, Box::new(&value))
    }

    /// Delete value at this pointer
    pub fn clear_here(cursor_addr: NP_Cursor_Addr, memory: &NP_Memory) -> bool {

        if cursor_addr.is_virtual == false {
            let cursor = memory.get_cursor_data(&cursor_addr).unwrap();
            if cursor.address_value != 0 {
                cursor.address_value = 0;
                memory.write_address(cursor.address, 0);
                true
            } else {
                false
            }
        } else {
            false     
        }
    }

    pub fn get_json<'json>(cursor_addr: NP_Cursor_Addr, memory: &'json NP_Memory<'json>, path: &[&str]) -> NP_JSON {
        
        match NP_Cursor::select(cursor_addr, memory, path, 0) {
            Ok(new_addr) => {
                if let Some(x) = new_addr {
                    NP_Cursor::json_encode(x, memory)
                } else {
                    NP_JSON::Null
                }
            },
            Err(_e) => {
                NP_JSON::Null
            }
        }
    }

    /// Exports this pointer and all it's descendants into a JSON object.
    /// This will create a copy of the underlying data and return default values where there isn't data.
    /// 
    pub fn json_encode(cursor_addr: NP_Cursor_Addr, memory: &NP_Memory) -> NP_JSON {

        let cursor = memory.get_cursor_data(&cursor_addr).unwrap();

        if cursor.address_value == 0 {
            return NP_JSON::Null;
        }

        match cursor.schema.get_type_key() {
            NP_TypeKeys::None           => { NP_JSON::Null },
            NP_TypeKeys::Any            => { NP_JSON::Null },
            NP_TypeKeys::UTF8String     => {    String::to_json(cursor_addr, memory) },
            NP_TypeKeys::Bytes          => {  NP_Bytes::to_json(cursor_addr, memory) },
            NP_TypeKeys::Int8           => {        i8::to_json(cursor_addr, memory) },
            NP_TypeKeys::Int16          => {       i16::to_json(cursor_addr, memory) },
            NP_TypeKeys::Int32          => {       i32::to_json(cursor_addr, memory) },
            NP_TypeKeys::Int64          => {       i64::to_json(cursor_addr, memory) },
            NP_TypeKeys::Uint8          => {        u8::to_json(cursor_addr, memory) },
            NP_TypeKeys::Uint16         => {       u16::to_json(cursor_addr, memory) },
            NP_TypeKeys::Uint32         => {       u32::to_json(cursor_addr, memory) },
            NP_TypeKeys::Uint64         => {       u64::to_json(cursor_addr, memory) },
            NP_TypeKeys::Float          => {       f32::to_json(cursor_addr, memory) },
            NP_TypeKeys::Double         => {       f64::to_json(cursor_addr, memory) },
            NP_TypeKeys::Decimal        => {    NP_Dec::to_json(cursor_addr, memory) },
            NP_TypeKeys::Boolean        => {      bool::to_json(cursor_addr, memory) },
            NP_TypeKeys::Geo            => {    NP_Geo::to_json(cursor_addr, memory) },
            NP_TypeKeys::Uuid           => {   NP_UUID::to_json(cursor_addr, memory) },
            NP_TypeKeys::Ulid           => {   NP_ULID::to_json(cursor_addr, memory) },
            NP_TypeKeys::Date           => {   NP_Date::to_json(cursor_addr, memory) },
            NP_TypeKeys::Enum           => { NP_Option::to_json(cursor_addr, memory) },
            NP_TypeKeys::Table          => {  NP_Table::to_json(cursor_addr, memory) },
            NP_TypeKeys::Map            => {    NP_Map::to_json(cursor_addr, memory) },
            NP_TypeKeys::List           => {   NP_List::to_json(cursor_addr, memory) },
            NP_TypeKeys::Tuple          => {  NP_Tuple::to_json(cursor_addr, memory) }
        }
    }

    pub fn compact(from_cursor: NP_Cursor_Addr, from_memory: &NP_Memory, to_cursor: NP_Cursor_Addr, to_memory: &NP_Memory) -> Result<NP_Cursor_Addr, NP_Error> {

        let cursor = from_memory.get_cursor_data(&from_cursor).unwrap();

        if cursor.address_value == 0 {
            return Ok(to_cursor);
        }

        match **cursor.schema {
            NP_Parsed_Schema::Any        { sortable: _, i:_ }                        => { Ok(to_cursor) }
            NP_Parsed_Schema::UTF8String { sortable: _, i:_, size:_, default:_ }     => {    String::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_Parsed_Schema::Bytes      { sortable: _, i:_, size:_, default:_ }     => {  NP_Bytes::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_Parsed_Schema::Int8       { sortable: _, i:_, default: _ }            => {        i8::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_Parsed_Schema::Int16      { sortable: _, i:_ , default: _ }           => {       i16::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_Parsed_Schema::Int32      { sortable: _, i:_ , default: _ }           => {       i32::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_Parsed_Schema::Int64      { sortable: _, i:_ , default: _ }           => {       i64::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_Parsed_Schema::Uint8      { sortable: _, i:_ , default: _ }           => {        u8::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_Parsed_Schema::Uint16     { sortable: _, i:_ , default: _ }           => {       u16::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_Parsed_Schema::Uint32     { sortable: _, i:_ , default: _ }           => {       u32::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_Parsed_Schema::Uint64     { sortable: _, i:_ , default: _ }           => {       u64::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_Parsed_Schema::Float      { sortable: _, i:_ , default: _ }           => {       f32::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_Parsed_Schema::Double     { sortable: _, i:_ , default: _ }           => {       f64::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_Parsed_Schema::Decimal    { sortable: _, i:_, exp:_, default:_ }      => {    NP_Dec::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_Parsed_Schema::Boolean    { sortable: _, i:_, default:_ }             => {      bool::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_Parsed_Schema::Geo        { sortable: _, i:_, default:_, size:_ }     => {    NP_Geo::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_Parsed_Schema::Uuid       { sortable: _, i:_ }                        => {   NP_UUID::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_Parsed_Schema::Ulid       { sortable: _, i:_ }                        => {   NP_ULID::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_Parsed_Schema::Date       { sortable: _, i:_, default:_ }             => {   NP_Date::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_Parsed_Schema::Enum       { sortable: _, i:_, default:_, choices: _ } => { NP_Option::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_Parsed_Schema::Table      { sortable: _, i:_, columns:_ }             => {  NP_Table::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_Parsed_Schema::Map        { sortable: _, i:_, value:_ }               => {    NP_Map::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_Parsed_Schema::List       { sortable: _, i:_, of:_ }                  => {   NP_List::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_Parsed_Schema::Tuple      { sortable: _, i:_, values:_ }              => {  NP_Tuple::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            _ => { panic!() }
        }
    }

    pub fn set_default(cursor_addr: NP_Cursor_Addr, memory: &NP_Memory) -> Result<(), NP_Error> {

        let cursor = memory.get_cursor_data(&cursor_addr).unwrap();

        match cursor.schema.get_type_key() {
            NP_TypeKeys::None        => { },
            NP_TypeKeys::Any         => { },
            NP_TypeKeys::Table       => { },
            NP_TypeKeys::Map         => { },
            NP_TypeKeys::List        => { },
            NP_TypeKeys::Tuple       => { },
            NP_TypeKeys::UTF8String  => {     String::set_value(cursor_addr, memory, Box::new(&String::default()))?; },
            NP_TypeKeys::Bytes       => {   NP_Bytes::set_value(cursor_addr, memory, Box::new(&NP_Bytes::default()))?; },
            NP_TypeKeys::Int8        => {         i8::set_value(cursor_addr, memory, Box::new(&i8::default()))?; },
            NP_TypeKeys::Int16       => {        i16::set_value(cursor_addr, memory, Box::new(&i16::default()))?; },
            NP_TypeKeys::Int32       => {        i32::set_value(cursor_addr, memory, Box::new(&i32::default()))?; },
            NP_TypeKeys::Int64       => {        i64::set_value(cursor_addr, memory, Box::new(&i64::default()))?; },
            NP_TypeKeys::Uint8       => {         u8::set_value(cursor_addr, memory, Box::new(&u8::default()))?; },
            NP_TypeKeys::Uint16      => {        u16::set_value(cursor_addr, memory, Box::new(&u16::default()))?; },
            NP_TypeKeys::Uint32      => {        u32::set_value(cursor_addr, memory, Box::new(&u32::default()))?; },
            NP_TypeKeys::Uint64      => {        u64::set_value(cursor_addr, memory, Box::new(&u64::default()))?; },
            NP_TypeKeys::Float       => {        f32::set_value(cursor_addr, memory, Box::new(&f32::default()))?; },
            NP_TypeKeys::Double      => {        f64::set_value(cursor_addr, memory, Box::new(&f64::default()))?; },
            NP_TypeKeys::Decimal     => {     NP_Dec::set_value(cursor_addr, memory, Box::new(&NP_Dec::default()))?; },
            NP_TypeKeys::Boolean     => {       bool::set_value(cursor_addr, memory, Box::new(&bool::default()))?; },
            NP_TypeKeys::Geo         => {     NP_Geo::set_value(cursor_addr, memory, Box::new(&NP_Geo::default()))?; },
            NP_TypeKeys::Uuid        => {    NP_UUID::set_value(cursor_addr, memory, Box::new(&NP_UUID::default()))?; },
            NP_TypeKeys::Ulid        => {    NP_ULID::set_value(cursor_addr, memory, Box::new(&NP_ULID::default()))?; },
            NP_TypeKeys::Date        => {    NP_Date::set_value(cursor_addr, memory, Box::new(&NP_Date::default()))?; },
            NP_TypeKeys::Enum        => {  NP_Option::set_value(cursor_addr, memory, Box::new(&NP_Option::default()))?; }
        };

        Ok(())
    }

        /// Calculate the number of bytes used by this pointer and it's descendants.
    /// 
    pub fn calc_size(cursor_addr: NP_Cursor_Addr, memory: &NP_Memory) -> Result<usize, NP_Error> {

        let cursor = memory.get_cursor_data(&cursor_addr).unwrap();

        // no pointer, no size
        if cursor.address == 0 {
            return Ok(0);
        }

        // size of pointer
        let base_size = memory.ptr_size(&cursor);

        // pointer is in buffer but has no value set
        if cursor.address_value == 0 { // no value, just base size
            return Ok(base_size);
        }

        // get the size of the value based on schema
        let type_size = match cursor.schema.get_type_key() {
            NP_TypeKeys::None         => { Ok(0) },
            NP_TypeKeys::Any          => { Ok(0) },
            NP_TypeKeys::UTF8String   => {    String::get_size(cursor_addr, memory) },
            NP_TypeKeys::Bytes        => {  NP_Bytes::get_size(cursor_addr, memory) },
            NP_TypeKeys::Int8         => {        i8::get_size(cursor_addr, memory) },
            NP_TypeKeys::Int16        => {       i16::get_size(cursor_addr, memory) },
            NP_TypeKeys::Int32        => {       i32::get_size(cursor_addr, memory) },
            NP_TypeKeys::Int64        => {       i64::get_size(cursor_addr, memory) },
            NP_TypeKeys::Uint8        => {        u8::get_size(cursor_addr, memory) },
            NP_TypeKeys::Uint16       => {       u16::get_size(cursor_addr, memory) },
            NP_TypeKeys::Uint32       => {       u32::get_size(cursor_addr, memory) },
            NP_TypeKeys::Uint64       => {       u64::get_size(cursor_addr, memory) },
            NP_TypeKeys::Float        => {       f32::get_size(cursor_addr, memory) },
            NP_TypeKeys::Double       => {       f64::get_size(cursor_addr, memory) },
            NP_TypeKeys::Decimal      => {    NP_Dec::get_size(cursor_addr, memory) },
            NP_TypeKeys::Boolean      => {      bool::get_size(cursor_addr, memory) },
            NP_TypeKeys::Geo          => {    NP_Geo::get_size(cursor_addr, memory) },
            NP_TypeKeys::Uuid         => {   NP_UUID::get_size(cursor_addr, memory) },
            NP_TypeKeys::Ulid         => {   NP_ULID::get_size(cursor_addr, memory) },
            NP_TypeKeys::Date         => {   NP_Date::get_size(cursor_addr, memory) },
            NP_TypeKeys::Enum         => { NP_Option::get_size(cursor_addr, memory) },
            NP_TypeKeys::Table        => {  NP_Table::get_size(cursor_addr, memory) },
            NP_TypeKeys::Map          => {    NP_Map::get_size(cursor_addr, memory) },
            NP_TypeKeys::List         => {   NP_List::get_size(cursor_addr, memory) },
            NP_TypeKeys::Tuple        => {  NP_Tuple::get_size(cursor_addr, memory) }
        }?;

        Ok(type_size + base_size)
    }
}



/// This trait is used to implement types as NoProto buffer types.
/// This includes all the type data, encoding and decoding methods.
#[doc(hidden)]
pub trait NP_Value<'value> {

    /// Get the type information for this type (static)
    /// 
    fn type_idx() -> (u8, String, NP_TypeKeys);

    /// Get the type information for this type (instance)
    /// 
    fn self_type_idx(&self) -> (u8, String, NP_TypeKeys);

    /// Convert the schema byte array for this type into JSON
    /// 
    fn schema_to_json(_schema_ptr: &NP_Parsed_Schema)-> Result<NP_JSON, NP_Error>;

    /// Set the value of this scalar into the buffer
    /// 
    fn set_value(_cursor: NP_Cursor_Addr, _memory: &NP_Memory, _value: Box<&Self>) -> Result<NP_Cursor_Addr, NP_Error>  where Self: NP_Value<'value> {
        let message = "This type doesn't support set_value!".to_owned();
        Err(NP_Error::new(message.as_str()))
    }

    /// Pull the data from the buffer and convert into type
    /// 
    fn into_value(_cursor: NP_Cursor_Addr, _memory: &'value NP_Memory) -> Result<Option<Box<Self>>, NP_Error> {
        let message = "This type doesn't support into!".to_owned();
        Err(NP_Error::new(message.as_str()))
    }

    /// Convert this type into a JSON value (recursive for collections)
    /// 
    fn to_json(_cursor: NP_Cursor_Addr, _memory: &'value NP_Memory) -> NP_JSON;

    /// Calculate the size of this pointer and it's children (recursive for collections)
    /// 
    fn get_size(_cursor: NP_Cursor_Addr, _memory: &'value NP_Memory) -> Result<usize, NP_Error>;
    
    /// Handle copying from old pointer/buffer to new pointer/buffer (recursive for collections)
    /// 
    fn do_compact(from_cursor: NP_Cursor_Addr, from_memory: &'value NP_Memory, to_cursor: NP_Cursor_Addr, to_memory: &'value NP_Memory) -> Result<NP_Cursor_Addr, NP_Error> where Self: NP_Value<'value> {

        match Self::into_value(from_cursor, from_memory)? {
            Some(x) => {
                return Self::set_value(to_cursor, to_memory, Box::new(&*x));
            },
            None => { }
        }

        Ok(to_cursor)
    }

    /// Get the default schema value for this type
    /// 
    fn schema_default(_schema: &NP_Parsed_Schema) -> Option<Box<Self>>;

    /// Parse JSON schema into schema
    ///
    fn from_json_to_schema(_json_schema: &NP_JSON) -> Result<Option<(Vec<u8>, NP_Parsed_Schema)>, NP_Error>;

    /// Parse bytes into schema
    /// 
    fn from_bytes_to_schema(_address: usize, _bytes: &Vec<u8>) -> NP_Parsed_Schema;
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NP_Cursor_Kinds {
    None,
    Standard,   // u32(4 bytes [4]), u16(2 bytes [2])

    Map,        // u32(4 bytes [4]), u16(2 bytes [2])
    Table,      // u32(4 bytes [4]), u16(2 bytes [2])
    List,       // u32(4 bytes [4]), u16(2 bytes [2])
    Tuple,      // u32(4 bytes [4]), u16(2 bytes [2])

    // collection itemsf
    MapItem,    // [addr | next | key] u32(12 bytes  [4, 4, 4]),  u16(6 bytes [2, 2, 2]), 
    TableItem,  // [addr | next | i: u8]  u32(9  bytes  [4, 4, 1]),  u16(5 bytes [2, 2, 1]),   
    ListItem,   // [addr | next | i: u16] u32(10 bytes  [4, 4, 2]),  u16(6 bytes [2, 2, 2]),
    TupleItem   // [addr]u32(4 bytes  [4]),  u16(2 bytes [2])           
}


/*
// stores the different kinds of pointers and the details for each pointer
#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub enum NP_PtrKinds {
    None,
    // scalar / collection
    Standard  { addr: usize },                    // u32(4 bytes [4]), u16(2 bytes [2])

    // collection items
    MapItem   { 
        addr: usize, next: usize, key: usize      // u32(12 bytes  [4, 4, 4]),  u16(6 bytes [2, 2, 2])
    }, 
    TableItem { 
        addr: usize, next: usize, i: u8           // u32(9  bytes  [4, 4, 1]),  u16(5 bytes [2, 2, 1])
    },   
    ListItem  { 
        addr: usize, next: usize, i: u16          // u32(10 bytes  [4, 4, 2]),  u16(6 bytes [2, 2, 2]),
    },   
    TupleItem  { 
        addr: usize, i: u8                        // u32(4 bytes  [4]),  u16(2 bytes [2])
    },                
}


impl NP_PtrKinds {

    /// Get the address of the value for this pointer
    pub fn get_value_addr(&self) -> usize {
        match self {
            NP_PtrKinds::None                                        =>    { 0 },
            NP_PtrKinds::Standard  { addr }                   =>    { *addr },
            NP_PtrKinds::MapItem   { addr, key: _,  next: _ } =>    { *addr },
            NP_PtrKinds::TableItem { addr, i: _,    next: _ } =>    { *addr },
            NP_PtrKinds::ListItem  { addr, i:_ ,    next: _ } =>    { *addr },
            NP_PtrKinds::TupleItem  { addr, i:_  }            =>    { *addr }
        }
    }
}


/// The base data type, all information is stored/retrieved against pointers
/// 
/// Each pointer represents at least a 16 or 32 bit unsigned integer that is either zero for no value or points to an offset in the buffer.  All pointer addresses are zero based against the beginning of the buffer.
///  
/// 
/// 
#[doc(hidden)]
#[derive(Debug)]
pub struct NP_Ptr<'ptr> {
    /// the kind of pointer this is (standard, list item, map item, etc).  Includes value address
    pub kind: NP_PtrKinds, 
    /// schema stores the *actual* schema data for this pointer, regardless of type casting
    pub schema: &'ptr Box<NP_Parsed_Schema>, 
    /// pointer address in buffer 
    pub address: usize,
    /// the underlying buffer this pointer is a part of
    pub memory: &'ptr NP_Memory,
    /// If this is a collection pointer, data about it's parent is here
    pub parent: NP_Ptr_Collection<'ptr>,
    /// If this is a collection pointer, more data about the location of this pointer
    pub helper: NP_Iterator_Helper<'ptr>
}

#[doc(hidden)]
#[derive(Debug, Clone)]
pub enum NP_Ptr_Collection<'coll> {
    None,
    List { address: usize, head: usize, tail: usize },
    Map { address: usize, head: usize, length: u16 },
    Table { address: usize, head: usize, schema: &'coll Box<NP_Parsed_Schema> },
    Tuple { address: usize, length: usize, schema: &'coll Box<NP_Parsed_Schema> }
}

#[doc(hidden)]
#[derive(Debug, Clone)]
pub enum NP_Iterator_Helper<'it> {
    None,
    List  { index: u16, prev_addr: usize, next_addr: usize, next_index: u16 },
    Table { index: u8, column: &'it str, prev_addr: usize, skip_step: bool },
    Map   { key_addr: usize , prev_addr: usize, key: Option<String> },
    Tuple { index: u8 }
}

impl<'it> NP_Iterator_Helper <'it> {
    /// Clone iterator helper
    pub fn clone(&self) -> Self {
        match self {
            NP_Iterator_Helper::None => NP_Iterator_Helper::None,
            NP_Iterator_Helper::List { index, prev_addr, next_index, next_addr} => {
                NP_Iterator_Helper::List { index: *index, prev_addr: *prev_addr, next_index: *next_index, next_addr: *next_addr}
            },
            NP_Iterator_Helper::Table { index, column, prev_addr, skip_step} => {
                NP_Iterator_Helper::Table { index: *index, column: *column, prev_addr: *prev_addr, skip_step: *skip_step}
            },
            NP_Iterator_Helper::Map { key_addr, prev_addr, key} => {
                NP_Iterator_Helper::Map { key_addr: *key_addr, prev_addr: *prev_addr, key: key.clone()}
            },
            NP_Iterator_Helper::Tuple { index } => {
                NP_Iterator_Helper::Tuple { index: *index }
            }
        }
    }
}

impl<'ptr> NP_Ptr<'ptr> {

    /// Retrieves the value at this pointer, only useful for scalar values (not collections).
    /// 
    pub fn get_here<T>(&'ptr self) -> Result<Option<T>, NP_Error> where T: Default + NP_Value<'ptr> {
        
        Ok(match T::into_value(&self)? {
            Some (x) => {
                Some(*x)
            },
            None => {
                match T::schema_default(&self.schema) {
                    Some(x) => Some(*x),
                    None => None
                }
            }
        })
    }  

    /// clone just the essential elements
    pub fn lite_clone(&self) -> Self {
        NP_Ptr {
            kind: NP_PtrKinds::None,
            schema: self.schema,
            address: 0,
            memory: (&self.memory),
            parent: NP_Ptr_Collection::None,
            helper: NP_Iterator_Helper::None
        }
    }

    /// Clone this pointer
    /// 
    pub fn clone(&self) -> Self {
        NP_Ptr {
            kind: self.kind,
            schema: self.schema,
            address: self.address,
            memory: (&self.memory),
            parent: self.parent.clone(),
            helper: self.helper.clone()
        }
    }

    /// Sets the value for this pointer, only works for scalar types (not collection types).
    /// 
    pub fn set_here<T>(&'ptr mut self, value: T) -> Result<(), NP_Error> where T: Default + NP_Value<'ptr> {
        T::set_value(self, Box::new(&value))
    }

    /// Create new standard pointer
    /// 
    pub fn _new_standard_ptr(address: usize, schema: &'ptr Box<NP_Parsed_Schema>, memory: &'ptr NP_Memory) -> Self {

        NP_Ptr {
            address: address,
            kind: NP_PtrKinds::Standard { addr: memory.read_address(address) },
            memory: memory,
            schema: schema,
            parent: NP_Ptr_Collection::None,
            helper: NP_Iterator_Helper::None
        }
    }

    /// Create new collection item pointer
    /// 
    pub fn _new_collection_item_ptr(address: usize, schema: &'ptr Box<NP_Parsed_Schema>, memory: &'ptr NP_Memory, parent: NP_Ptr_Collection<'ptr>, helper: NP_Iterator_Helper<'ptr>) -> Self {
        let b_bytes = &memory.read_bytes();

        NP_Ptr {
            address: address,
            kind: match parent {
                NP_Ptr_Collection::Table { address: _, head: _, schema: _} => {
                    NP_PtrKinds::TableItem { 
                        addr:  memory.read_address(address),
                        next:  memory.read_address_offset(address, 4, 2, 1),
                        i: if address == 0 { 0 } else { match &memory.size {
                            NP_Size::U32 => b_bytes[address + 8],
                            NP_Size::U16 => b_bytes[address + 4],
                            NP_Size::U8 => b_bytes[address + 2]
                        }},
                    }
                },
                NP_Ptr_Collection::List { address: _, head: _, tail: _} => {
                    NP_PtrKinds::ListItem { 
                        addr:  memory.read_address(address),
                        next:  memory.read_address_offset(address,  4, 2, 1),
                        i: if address == 0 { 0 } else { match &memory.size {
                            NP_Size::U32 => u16::from_be_bytes(*memory.get_2_bytes(address + 8).unwrap_or(&[0; 2])),
                            NP_Size::U16 => u16::from_be_bytes(*memory.get_2_bytes(address + 4).unwrap_or(&[0; 2])),
                            NP_Size::U8 => u8::from_be_bytes([memory.get_1_byte(address + 2).unwrap_or(0)]) as u16
                        }}
                    }
                },
                NP_Ptr_Collection::Map { address: _, head: _, length: _} => {
                    NP_PtrKinds::MapItem { 
                        addr:  memory.read_address(address),
                        next:  memory.read_address_offset(address,  4, 2, 1),
                        key:   memory.read_address_offset(address, 8, 4, 2)
                    }
                },
                _ => { panic!() }
            },
            memory: memory,
            schema: schema,
            parent,
            helper
        }
    }

    /// read kind data from buffer
    pub fn read_kind(address: usize, memory: &NP_Memory, parent: &NP_Ptr_Collection<'ptr>) -> NP_PtrKinds {
        let b_bytes = &memory.read_bytes();

        match parent {
            NP_Ptr_Collection::None => {
                NP_PtrKinds::Standard {
                    addr:  memory.read_address(address),
                }
            },
            NP_Ptr_Collection::Tuple {address: _, length: _, schema: _} => {
                NP_PtrKinds::TupleItem {
                    addr:  memory.read_address(address),
                    i: 0
                }
            },
            NP_Ptr_Collection::Table { address: _, head: _, schema: _} => {
                NP_PtrKinds::TableItem { 
                    addr:  memory.read_address(address),
                    next:  memory.read_address_offset(address, 4, 2, 1),
                    i: if address == 0 { 0 } else { match &memory.size {
                        NP_Size::U32 => b_bytes[address + 8],
                        NP_Size::U16 => b_bytes[address + 4],
                        NP_Size::U8 => b_bytes[address + 2]
                    }},
                }
            },
            NP_Ptr_Collection::List { address: _, head: _, tail: _} => {
                NP_PtrKinds::ListItem { 
                    addr:  memory.read_address(address),
                    next:  memory.read_address_offset(address,  4, 2, 1),
                    i: if address == 0 { 0 } else { match &memory.size {
                        NP_Size::U32 => u16::from_be_bytes(*memory.get_2_bytes(address + 8).unwrap_or(&[0; 2])),
                        NP_Size::U16 => u16::from_be_bytes(*memory.get_2_bytes(address + 4).unwrap_or(&[0; 2])),
                        NP_Size::U8 => u8::from_be_bytes([memory.get_1_byte(address + 2).unwrap_or(0)]) as u16
                    }}
                }
            },
            NP_Ptr_Collection::Map { address: _, head: _, length: _} => {
                NP_PtrKinds::MapItem { 
                    addr:  memory.read_address(address),
                    next:  memory.read_address_offset(address,  4, 2, 1),
                    key:   memory.read_address_offset(address, 8, 4, 2)
                }
            }
        }
    }


    /// Check if there is any value set at this pointer
    /// 
    pub fn has_value(&self) -> bool {

        if self.address == 0 || self.kind.get_value_addr() == 0 {
            return false;
        }

        return true;
    }


    /// Clear / delete the value at this pointer.  This is just clears the value address, so it doesn't actually remove items from the buffer.  Also if this is called on a collection type, all children of the collection will also be cleared.
    /// 
    /// If you clear a large object it's probably a good idea to run compaction to recover the free space.
    /// 
    pub fn clear_here(target: &mut NP_Ptr<'ptr>) -> bool {
        if target.address != 0 {
            target.memory.set_value_address(target.address, 0, &target.kind);
            true
        } else {
            false
        }
    }

    /// Deep delete a value
    /// 
    pub fn _deep_delete(target: &mut NP_Ptr<'ptr>, path: &[&str], path_index: usize) -> Result<bool, NP_Error> {

        NP_Ptr::_deep_get(target, path, path_index)?;
        Ok(NP_Ptr::clear_here(target))
    }

    /// Create a path to a pointer and provide the pointer
    /// 

    pub fn _deep_set(ptr: &mut NP_Ptr<'ptr>, path: &[&str], path_index: usize) -> Result<(), NP_Error> {

        if path.len() == path_index {
            return Ok(());
        }

        let type_data = &ptr.schema.into_type_data();

        match type_data.2 {
            NP_TypeKeys::Table => {
                NP_Table::select_to_ptr(ptr, &path[path_index], None)?;
                NP_Table::commit_pointer(ptr)?;
                NP_Ptr::_deep_set(ptr, path, path_index + 1)
            },
            NP_TypeKeys::Map => {

                NP_Map::select_to_ptr(ptr, &path[path_index], false)?;
                NP_Map::commit_pointer(ptr)?;
                NP_Ptr::_deep_set(ptr, path, path_index + 1)
            },
            NP_TypeKeys::List => {

                let list_key_int = (&path[path_index]).parse::<u16>();
                match list_key_int {
                    Ok(x) => {
                        NP_List::select_to_ptr(ptr, x)?;
                        NP_List::commit_pointer(ptr)?;
                        NP_Ptr::_deep_set(ptr, path, path_index + 1)
                    },
                    Err(_e) => {
                        let mut err = String::from("Can't query list with string, need number! Path: \n");
                        err.push_str(print_path(&path, path_index).as_str());
                        Err(NP_Error::new(err))
                    }
                }
            },
            NP_TypeKeys::Tuple => {

                let list_key = &path[path_index];
                let list_key_int = list_key.parse::<u8>();
                match list_key_int {
                    Ok(x) => {
                        NP_Tuple::select_to_ptr(ptr, x)?;
                        NP_Ptr::_deep_set(ptr, path, path_index + 1)
                    },
                    Err(_e) => {
                        let mut err = String::from("Can't query tuple with string, need number! Path: \n");
                        err.push_str(print_path(&path, path_index).as_str());
                        Err(NP_Error::new(err))
                    }
                }

            },
            _ => { // scalar type
                
                Ok(())
            }
        }
    }

    /// Deep set a value
    /// 
    #[allow(unused_mut)]
    pub fn _deep_set_value<X>(target: &mut NP_Ptr<'ptr>, path: &[&str], path_index: usize, value: X) -> Result<(), NP_Error> where X: NP_Value<'ptr> + Default {

        // let mut target_ptr = NP_Ptr::_new_standard_ptr(0, &self.schema, (&self.memory));
        NP_Ptr::_deep_set(target, path, path_index)?;

        let type_data = target.schema.into_type_data();

        // if schema is ANY then allow any type to set this value
        // otherwise make sure the schema and type match
        if type_data.0 != NP_Any::type_idx().0 && type_data.0 != X::type_idx().0 {
            let mut err = "TypeError: Attempted to set value for type (".to_owned();
            err.push_str(X::type_idx().1.as_str());
            err.push_str(") into schema of type (");
            err.push_str(type_data.1.as_str());
            err.push_str("}\n");
            return Err(NP_Error::new(err));
        }

        X::set_value(target, Box::new(&value))?;

        Ok(())

    }

    /// deep get with type
    /// 
    pub fn _deep_get_type<T>(target: &mut NP_Ptr<'ptr>, path: &[&str]) -> Result<Option<Box<T>>, NP_Error> where T: NP_Value<'ptr> + Default {
        println!("1 {:?}", target);
        
        NP_Ptr::_deep_get(target, path, 0)?;

        println!("{:?}", target);

        if target.schema.into_type_data().0 != T::type_idx().0 {
            let mut err = "TypeError: Attempted to set value for type (".to_owned();
            err.push_str(T::type_idx().1.as_str());
            err.push_str(") into schema of type (");
            err.push_str(target.schema.into_type_data().1.as_str());
            err.push_str(")\n");
            return Err(NP_Error::new(err));
        }
        if target.has_value() {
            T::into_value(&target)
        } else {
            Ok(T::schema_default(target.schema))
        }
     
    }

    /// Deep get a value
    /// 
    pub fn _deep_get(ptr: &mut NP_Ptr<'ptr>, path: &[&str], path_index: usize) -> Result<bool, NP_Error> {

        if path.len() == path_index {
            return Ok(true);
        }

        let type_data = ptr.schema.into_type_data();

        match type_data.2 {
            NP_TypeKeys::Table => {
                NP_Table::select_to_ptr(ptr, &path[path_index], None)?;
                NP_Ptr::_deep_get(ptr, path, path_index + 1)
            },
            NP_TypeKeys::Map => {
                NP_Map::select_to_ptr(ptr, &path[path_index], false)?;
                NP_Ptr::_deep_get(ptr, path, path_index + 1)
            },
            NP_TypeKeys::List => {
            
                let list_key = &path[path_index];
                let list_key_int = list_key.parse::<u16>();
                match list_key_int {
                    Ok(x) => {
                        NP_List::select_to_ptr(ptr, x)?;
                        NP_Ptr::_deep_get(ptr, path, path_index + 1)
                    },
                    Err(_e) => {
                        let mut err = String::from("Can't query list with string, need number! Path: \n");
                        err.push_str(print_path(&path, path_index).as_str());
                        Err(NP_Error::new(err))
                    }
                }
           
            },
            NP_TypeKeys::Tuple => {

                let list_key = &path[path_index];
                let list_key_int = list_key.parse::<u8>();
                match list_key_int {
                    Ok(x) => {
                        NP_Tuple::select_to_ptr(ptr, x)?;
                        NP_Ptr::_deep_get(ptr, path, path_index + 1)
                    },
                    Err(_e) => {
                        let mut err = String::from("Can't query tuple with string, need number! Path: \n");
                        err.push_str(print_path(&path, path_index).as_str());
                        Err(NP_Error::new(err))
                    }
                }
                 
            },
            _ => { 
                // we're not at the end of the select path but we've reached a scalar value
                // so the select has failed to find anything
                return Ok(false); 
            }
        }

        // Ok(None)
    }
    



    #[doc(hidden)]

}
*/

/*
// unsigned integer size:        0 to (2^i) -1
//   signed integer size: -2^(i-1) to  2^(i-1) 
*/