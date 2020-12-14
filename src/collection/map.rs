use alloc::string::String;
use crate::{hashmap::{NP_HashMap, SEED, murmurhash3_x86_32}, pointer::NP_Map_Bytes, schema::NP_Schema_Addr};
use crate::pointer::NP_Cursor;
use crate::{json_flex::JSMAP};
use crate::pointer::{NP_Value};
use crate::{memory::{NP_Memory}, schema::{NP_Schema, NP_TypeKeys, NP_Parsed_Schema}, error::NP_Error, json_flex::NP_JSON};

use alloc::string::ToString;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::borrow::ToOwned;
use core::{str::from_utf8_unchecked, hint::unreachable_unchecked};

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
struct Map_Item<'item> {
    key: &'item str,
    buff_addr: usize
}

/// The map type.
/// 
#[doc(hidden)]
pub struct NP_Map<'map> { 
    current: Option<Map_Item<'map>>,
    previous: Option<Map_Item<'map>>,
    key: &'map str,
    head: Option<Map_Item<'map>>,
    map: NP_Cursor,
    value_of: usize
}

impl<'map> NP_Map<'map> {

    #[inline(always)]
    pub fn select(map_cursor: NP_Cursor, key: &str, schema_only: bool, memory: &'map NP_Memory) -> Result<NP_Cursor, NP_Error> {

        let value_of = match memory.schema[map_cursor.schema_addr] {
            NP_Parsed_Schema::Map { value, .. } => value,
            _ => unsafe { panic!() }
        };

        if schema_only {
            return Ok(NP_Cursor::new(0, value_of, map_cursor.schema_addr))
        }

        let mut map_iter = Self::new_iter(&map_cursor, memory);

        // key is in map
        while let Some((ikey, item)) = map_iter.step_iter(memory) {
            if ikey == key {
                return Ok(item.clone())
            }
        }

        // key is not in map, make a new one
        Self::insert(&map_cursor, memory, key)
    }

    #[inline(always)]
    pub fn get_map<'get>(map_buff_addr: usize, memory: &'get NP_Memory<'get>) -> &'get mut NP_Map_Bytes {
        unsafe { &mut *(memory.write_bytes().as_ptr().add(map_buff_addr as usize) as *mut NP_Map_Bytes) }
    }

    #[inline(always)]
    pub fn new_iter(map_cursor: &NP_Cursor, memory: &'map NP_Memory) -> Self {

        let value_of = match memory.schema[map_cursor.schema_addr] {
            NP_Parsed_Schema::Map { value, .. } => value,
            _ => unsafe { panic!() }
        };

        if map_cursor.get_value(memory).get_addr_value() == 0 {
            return Self {
                current: None,
                previous: None,
                key: "",
                head: None,
                map: map_cursor.clone(),
                value_of
            }
        }

        let head_addr = Self::get_map(map_cursor.buff_addr, memory).get_head();

        let head_cursor = NP_Cursor::new(head_addr as usize, value_of, map_cursor.schema_addr);
        let head_cursor_value = head_cursor.get_value(memory);

        Self {
            current: None,
            previous: None,
            key: "",
            head: Some(Map_Item {
                key: head_cursor_value.get_key(memory),
                buff_addr: head_cursor.buff_addr 
            }),
            map: map_cursor.clone(),
            value_of
        }
    }

    #[inline(always)]
    pub fn step_iter(&mut self, memory: &'map NP_Memory<'map>) -> Option<(&'map str, NP_Cursor)> {
        
        match self.head {
            Some(head) => {

                match self.current {
                    Some(current) => { // subsequent iterations
                        let current_item = NP_Cursor::new(current.buff_addr, self.value_of, self.map.schema_addr);
                        let current_value = current_item.get_value(memory);
                        let next_value = current_value.get_next_addr() as usize;
                        if next_value == 0 { //nothing left to step
                            return None;
                        } else {
                            let next_value_cursor = NP_Cursor::new(next_value, self.value_of, self.map.schema_addr);
                            let next_value_value = next_value_cursor.get_value(memory);
                            self.previous = self.current.clone();
                            let key = next_value_value.get_key(memory);
                            self.current = Some(Map_Item { buff_addr: next_value, key: key });
                            return Some((key, next_value_cursor))
                        }
                    },
                    None => { // first iteration, get head
                        self.current = Some(head.clone());
                        return Some((head.key, NP_Cursor::new(head.buff_addr, self.value_of, self.map.schema_addr)))
                    }
                }
            },
            None => return None
        }


    }

    #[inline(always)]
    pub fn insert(map_cursor: &NP_Cursor, memory: &NP_Memory, key: &str) -> Result<NP_Cursor, NP_Error> {

        let value_of = match memory.schema[map_cursor.schema_addr] {
            NP_Parsed_Schema::Map { value, .. } => value,
            _ => unsafe { panic!() }
        };

        if key.len() >= 255 {
            return Err(NP_Error::new("Key length cannot be larger than 255 charecters!"));
        }

        let map_value = map_cursor.get_value(memory);

        let new_cursor_addr = memory.malloc_borrow(&[0u8; 6])?;
        let new_cursor = NP_Cursor::new(new_cursor_addr, value_of, map_cursor.schema_addr);
        let new_cursor_value = new_cursor.get_value(memory);

        // set key
        let key_item_addr = memory.malloc_borrow(&[key.len() as u8])?;
        memory.malloc_borrow(key.as_bytes())?;
        new_cursor_value.set_key_addr(key_item_addr as u16);

        let head = map_value.get_addr_value() as usize;

        // Set head of map to new cursor
        map_value.set_addr_value(new_cursor_addr as u16);

        if head != 0 { // set new cursors NEXT to old HEAD
            new_cursor_value.set_next_addr(head as u16);
        }

        Ok(new_cursor)
    }

    #[inline(always)]
    pub fn for_each<F>(cursor_addr: &NP_Cursor, memory: &'map NP_Memory, callback: &mut F) where F: FnMut((&str, NP_Cursor)) {

        let mut map_iter = Self::new_iter(cursor_addr, memory);

        while let Some((index, item)) = Self::step_iter(&mut map_iter, memory) {
            callback((index, item))
        }

    }

}

impl<'value> NP_Value<'value> for NP_Map<'value> {

    fn type_idx() -> (&'value str, NP_TypeKeys) { ("map", NP_TypeKeys::Map) }
    fn self_type_idx(&self) -> (&'value str, NP_TypeKeys) { ("map", NP_TypeKeys::Map) }
    
    fn schema_to_json(schema: &Vec<NP_Parsed_Schema>, address: usize)-> Result<NP_JSON, NP_Error> {
        let mut schema_json = JSMAP::new();
        schema_json.insert("type".to_owned(), NP_JSON::String(Self::type_idx().0.to_string()));

        let value_of = match schema[address] {
            NP_Parsed_Schema::Map { value, .. } => {
                value
            },
            _ => { unsafe { panic!() } }
        };

        schema_json.insert("value".to_owned(), NP_Schema::_type_to_json(schema, value_of)?);

        Ok(NP_JSON::Dictionary(schema_json))
    }

    fn get_size(cursor: &NP_Cursor, memory: &'value NP_Memory<'value>) -> Result<usize, NP_Error> {

        let c_value = cursor.get_value(memory);

        if c_value.get_addr_value() == 0 {
            return Ok(0) 
        }

        let mut acc_size = 0usize;

        Self::for_each(&cursor, memory, &mut |(_i, item)| {
            let key_size = item.get_value(memory).get_key_size(memory);
            acc_size += 1; // length byte
            acc_size += key_size;
            acc_size += NP_Cursor::calc_size(&item, memory).unwrap();
        });

        Ok(acc_size)
   
    }

    fn to_json(cursor: &NP_Cursor, memory: &'value NP_Memory) -> NP_JSON {

        let c_value = cursor.get_value(memory);

        if c_value.get_addr_value() == 0 {
            return NP_JSON::Null
        }

        let mut json_map = JSMAP::new();

        Self::for_each(&cursor, memory, &mut |(key, item)| {
            json_map.insert(String::from(key), NP_Cursor::json_encode(&item, memory));
        });

        NP_JSON::Dictionary(json_map)
   
    }

    fn do_compact(from_cursor: NP_Cursor, from_memory: &'value NP_Memory, to_cursor: NP_Cursor, to_memory: &'value NP_Memory) -> Result<NP_Cursor, NP_Error> where Self: 'value + Sized {

        let from_value = from_cursor.get_value(from_memory);

        if from_value.get_addr_value() == 0 {
            return Ok(to_cursor) 
        }

        let value_of = match from_memory.schema[from_cursor.schema_addr] {
            NP_Parsed_Schema::Map { value, .. } => value,
            _ => unsafe { panic!() }
        };

        Self::for_each(&from_cursor, from_memory,  &mut |(key, item)| {
            let new_item = Self::insert(&to_cursor, to_memory, key).unwrap();
            NP_Cursor::compact(item.clone(), from_memory, new_item, to_memory).unwrap();
        });

        Ok(to_cursor)
    }

    fn from_json_to_schema(mut schema: Vec<NP_Parsed_Schema>, json_schema: &Box<NP_JSON>) -> Result<(bool, Vec<u8>, Vec<NP_Parsed_Schema>), NP_Error> {
      
        let mut schema_data: Vec<u8> = Vec::new();
        schema_data.push(NP_TypeKeys::Map as u8);

        let value_addr = schema.len();
        schema.push(NP_Parsed_Schema::Map {
            i: NP_TypeKeys::Map,
            value: value_addr + 1,
            sortable: false
        });

        match json_schema["value"] {
            NP_JSON::Null => {
                return Err(NP_Error::new("Maps require a 'value' property that is a schema type!"))
            },
            _ => { }
        }

        
        let (_sortable, child_bytes, schema) = NP_Schema::from_json(schema, &Box::new(json_schema["value"].clone()))?;
        
        schema_data.extend(child_bytes);

        return Ok((false, schema_data, schema))

    }

    fn schema_default(_schema: &NP_Parsed_Schema) -> Option<Self> {
        None
    }

    fn from_bytes_to_schema(mut schema: Vec<NP_Parsed_Schema>, address: usize, bytes: &Vec<u8>) -> (bool, Vec<NP_Parsed_Schema>) {
        let of_addr = schema.len();
        schema.push(NP_Parsed_Schema::Map {
            i: NP_TypeKeys::Map,
            sortable: false,
            value: of_addr + 1
        });
        let (_sortable, schema) = NP_Schema::from_bytes(schema, address + 1, bytes);
        (false, schema)
    }
}


#[test]
fn schema_parsing_works() -> Result<(), NP_Error> {
    let schema = "{\"type\":\"map\",\"value\":{\"type\":\"string\"}}";
    let factory = crate::NP_Factory::new(schema)?;
    assert_eq!(schema, factory.schema.to_json()?.stringify());
    
    Ok(())
}

#[test]
fn set_clear_value_and_compaction_works() -> Result<(), NP_Error> {
    let schema = "{\"type\":\"map\",\"value\":{\"type\":\"string\"}}";
    let factory = crate::NP_Factory::new(schema)?;

    // compaction works
    let mut buffer = factory.empty_buffer(None);
    buffer.set(&["name"], "hello, world")?;
    assert_eq!(buffer.get::<&str>(&["name"])?, Some("hello, world"));
    assert_eq!(buffer.calc_bytes()?.after_compaction, buffer.calc_bytes()?.current_buffer);
    assert_eq!(buffer.calc_bytes()?.current_buffer, 27usize);
    buffer.del(&[])?;
    buffer.compact(None)?;
    assert_eq!(buffer.calc_bytes()?.current_buffer, 2usize);

    // values are preserved through compaction
    let mut buffer = factory.empty_buffer(None);
    buffer.set(&["name"], "hello, world")?;
    buffer.set(&["name2"], "hello, world2")?;
    assert_eq!(buffer.get::<&str>(&["name"])?, Some("hello, world"));
    assert_eq!(buffer.get::<&str>(&["name2"])?, Some("hello, world2"));
    assert_eq!(buffer.calc_bytes()?.current_buffer, 54usize);
    buffer.compact(None)?;
    assert_eq!(buffer.get::<&str>(&["name"])?, Some("hello, world"));
    assert_eq!(buffer.get::<&str>(&["name2"])?, Some("hello, world2"));
    assert_eq!(buffer.calc_bytes()?.current_buffer, 54usize);

    Ok(())
}