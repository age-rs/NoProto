//! Schemas are JSON used to declare the shape of buffer objects
//! 
//! No Proto Schemas are JSON objects that describe how the data in a buffer is stored and what types of data is stored.  Schemas are required to create buffers and each buffer is a descendant of the schema that created it.
//! 
//! Buffers are forever related to the schema that created them, buffers created from a given schema can only later be decoded, edited or compacted by that same schema.
//! 
//! Schemas are validated and sanity checked upon creation.  You cannot pass an invalid schema into a factory constructor and build/parse buffers with it.
//! 
//! Properties that are not part of the schema are ignored.
//! 
//! If you're familiar with Typescript, schemas can be described by this recursive interface:
//! ```typescript
//! interface NP_Schema {
//!     // table, string, bytes, etc
//!     type: string; 
//!     
//!     // used by string & bytes types
//!     size?: number;
//!     
//!     // used by decimal type, the number of decimal places every value has
//!     exp?: number;
//!     
//!     // used by tuple to indicite bytewise sorting of children
//!     sorted?: boolean;
//!     
//!     // used by list types
//!     of?: NP_Schema
//!     
//!     // used by map types
//!     value?: NP_Schema
//! 
//!     // used by tuple types
//!     values?: NP_Schema[]
//! 
//!     // used by table types
//!     columns?: [string, NP_Schema][]
//! 
//!     // used by option/enum types
//!     choices?: string[];
//! 
//!     // default value for this item
//!     default?: any;
//! }
//! ```
//! 
//! Schemas can be as simple as a single scalar type, for example a perfectly valid schema for a buffer that contains only a string:
//! ```json
//! {
//!     "type": "string"
//! }
//! ```
//! 
//! However, you will likely want to store more complicated objects, so that's easy to do as well.
//! ```json
//! {
//!     "type": "table",
//!     "columns": [
//!         ["userID",   {"type": "string"}], // userID column contains a string
//!         ["password", {"type": "string"}], // password column contains a string
//!         ["email",    {"type": "string"}], // email column contains a string
//!         ["age",      {"type": "u8"}]     // age column contains a Uint8 number (0 - 255)
//!     ]
//! }
//! ```
//! 
//! There are multiple collection types, and they can be nested.
//! 
//! For example, this is a list of tables.  Every item in the list is a table with two columns: id and title.  Both columns are a string type.
//! ```json
//! {
//!     "type": "list",
//!     "of": {
//!         "type": "table",
//!         "columns": [
//!             ["id",    {type: "string"}]
//!             ["title", {type: "string"}]
//!         ]
//!     }
//! }
//! ```
//! You can nest collections as much and however you'd like. Nesting is only limited by the address space of the buffer, so go crazy.
//! 
//! A list of strings is just as easy...
//! 
//! ```json
//! {
//!     "type": "list",
//!     "of": { type: "string" }
//! }
//! ```
//! 
//! Each type has trade offs associated with it.  The table and documentation below go into further detail.
//! 
//! ## Supported Data Types
//! 
//! | Type                                   | Rust Type / Struct                                                       |Bytewise Sorting  | Bytes (Size)   | Limits / Notes                                                           |
//! |----------------------------------------|--------------------------------------------------------------------------|------------------|----------------|--------------------------------------------------------------------------|
//! | [`table`](#table)                      | [`NP_Table`](../collection/table/struct.NP_Table.html)                   |𐄂                 | 2 bytes - ~4GB | Linked list with indexed keys that map against up to 255 named columns.  |
//! | [`list`](#list)                        | [`NP_List`](../collection/list/struct.NP_List.html)                      |𐄂                 | 4 bytes - ~4GB | Linked list with integer indexed values and  up to 65,535 items.         |
//! | [`map`](#map)                          | [`NP_Map`](../collection/map/struct.NP_Map.html)                         |𐄂                 | 2 bytes - ~4GB | Linked list with `Vec<u8>` keys.                                         |
//! | [`tuple`](#tuple)                      | [`NP_Tuple`](../collection/tuple/struct.NP_Tuple.html)                   |✓ *               | 2 bytes - ~4GB | Static sized collection of specific values.                              |
//! | [`any`](#any)                          | [`NP_Any`](../pointer/any/struct.NP_Any.html)                            |𐄂                 | 2 bytes - ~4GB | Generic type.                                                            |
//! | [`string`](#string)                    | [`String`](../pointer/string/index.html)                                 |✓ **              | 2 bytes - ~4GB | Utf-8 formatted string.                                                  |
//! | [`bytes`](#bytes)                      | [`NP_Bytes`](../pointer/bytes/struct.NP_Bytes.html)                      |✓ **              | 2 bytes - ~4GB | Arbitrary bytes.                                                         |
//! | [`int8`](#int8-int16-int32-int64)      | [`i8`](../pointer/numbers/index.html)                                    |✓                 | 1 byte         | -127 to 127                                                              |
//! | [`int16`](#int8-int16-int32-int64)     | [`i16`](../pointer/numbers/index.html)                                   |✓                 | 2 bytes        | -32,768 to 32,768                                                        |
//! | [`int32`](#int8-int16-int32-int64)     | [`i32`](../pointer/numbers/index.html)                                   |✓                 | 4 bytes        | -2,147,483,648 to 2,147,483,648                                          |
//! | [`int64`](#int8-int16-int32-int64)     | [`i64`](../pointer/numbers/index.html)                                   |✓                 | 8 bytes        | -9,223,372,036,854,775,808 to 9,223,372,036,854,775,808                  |
//! | [`uint8`](#uint8-uint16-uint32-uint64) | [`u8`](../pointer/numbers/index.html)                                    |✓                 | 1 byte         | 0 - 255                                                                  |
//! | [`uint16`](#uint8-uint16-uint32-uint64)| [`u16`](../pointer/numbers/index.html)                                   |✓                 | 2 bytes        | 0 - 65,535                                                               |
//! | [`uint32`](#uint8-uint16-uint32-uint64)| [`u32`](../pointer/numbers/index.html)                                   |✓                 | 4 bytes        | 0 - 4,294,967,295                                                        |
//! | [`uint64`](#uint8-uint16-uint32-uint64)| [`u64`](../pointer/numbers/index.html)                                   |✓                 | 8 bytes        | 0 - 18,446,744,073,709,551,616                                           |
//! | [`float`](#float-double)               | [`f32`](../pointer/numbers/index.html)                                   |𐄂                 | 4 bytes        | -3.4e38 to 3.4e38                                                        |
//! | [`double`](#float-double)              | [`f64`](../pointer/numbers/index.html)                                   |𐄂                 | 8 bytes        | -1.7e308 to 1.7e308                                                      |
//! | [`option`](#option)                    | [`NP_Option`](../pointer/option/struct.NP_Option.html)                   |✓                 | 1 byte         | Up to 255 string based options in schema.                                |
//! | [`bool`](#bool)                        | [`bool`](../pointer/bool/index.html)                                     |✓                 | 1 byte         |                                                                          |
//! | [`decimal`](#decimal)                  | [`NP_Dec`](../pointer/dec/struct.NP_Dec.html)                            |✓                 | 8 bytes        | Fixed point decimal number based on i64.                                 |
//! | [`geo4`](#geo4-geo8-geo16)             | [`NP_Geo`](../pointer/geo/struct.NP_Geo.html)                            |✓                 | 4 bytes        | 1.1km resolution (city) geographic coordinate                            |
//! | [`geo8`](#geo4-geo8-geo16)             | [`NP_Geo`](../pointer/geo/struct.NP_Geo.html)                            |✓                 | 8 bytes        | 11mm resolution (marble) geographic coordinate                           |
//! | [`geo16`](#geo4-geo8-geo16)            | [`NP_Geo`](../pointer/geo/struct.NP_Geo.html)                            |✓                 | 16 bytes       | 110 microns resolution (grain of sand) geographic coordinate             |
//! | [`ulid`](#ulid)                        | [`NP_ULID`](../pointer/ulid/struct.NP_ULID.html)                         |✓                 | 16 bytes       | 6 bytes for the timestamp, 10 bytes of randomness.                       |
//! | [`uuid`](#uuid)                        | [`NP_UUID`](../pointer/uuid/struct.NP_UUID.html)                         |✓                 | 16 bytes       | v4 UUID, 2e37 possible UUIDs                                             |
//! | [`date`](#date)                        | [`NP_Date`](../pointer/date/struct.NP_Date.html)                         |✓                 | 8 bytes        | Good to store unix epoch (in milliseconds) until the year 584,866,263    |
//!  
//! - \* `sorting` must be set to `true` in the schema for this object to enable sorting.
//! - \*\* String & Bytes can be bytewise sorted only if they have a `size` property in the schema
//! 
//! # Legend
//! 
//! **Bytewise Sorting**<br/>
//! Bytewise sorting means that two buffers can be compared at the byte level *without deserializing* and a correct ordering between the buffer's internal values will be found.  This is extremely useful for storing ordered keys in databases.
//! 
//! Each type has specific notes on wether it supports bytewise sorting and what things to consider if using it for that purpose.
//! 
//! You can sort by multiple types/values if a tuple is used.  The ordering of values in the tuple will determine the sort order.  For example if you have a tuple with types (A, B) the ordering will first sort by A, then B where A is identical.  This is true for any number of items, for example a tuple with types (A,B,C,D) will sort by D when A, B & C are identical.
//! 
//! **Compaction**<br/>
//! Campaction is an optional operation you can perform at any time on a buffer, typically used to recover free space.  NoProto Buffers are contiguous, growing arrays of bytes.  When you add or update a value sometimes additional memory is used and the old value is dereferenced, meaning the buffer is now occupying more space than it needs to.  This space can be recovered with compaction.  Compaction involves a recursive, full copy of all referenced & valid values of the buffer, it's an expensive operation that should be avoided.
//! 
//! Sometimes the space you can recover with compaction is minimal or you can craft your schema and upates in such a way that compactions are never needed, in these cases compaction can be avoided with little to no consequence.
//! 
//! Deleting a value will almost always mean space can be recovered with compaction, but updating values can have different outcomes to the space used depending on the type and options.
//! 
//! Each type will have notes on how updates can lead to wasted bytes and require compaction to recover the wasted space.
//! 
//! - [How do you run compaction on a buffer?](../buffer/struct.NP_Buffer.html#method.compact)
//! 
//! **Schema Mutations**<br/> 
//! Once a schema is created all the buffers it creates depend on that schema for reliable de/serialization, data access, and compaction.
//! 
//! There are safe ways you can mutate a schema after it's been created without breaking old buffers, however those updates are limited.  The safe mutations will be mentioned for each type, consider any other schema mutations unsafe.
//! 
//! Changing the `type` property of any value in the schame is unsafe.  It's only sometimes safe to modify properties besides `type`.
//! 
//! # Schema Types
//! 
//! Every schema type maps exactly to a native data type in your code.
//! 
//! ## table
//! Tables represnt a fixed number of named columns, with each column having it's own data type.
//! 
//! - **Bytewise Sorting**: Unsupported
//! - **Compaction**: Columns without values will be removed from the buffer durring compaction.  If a column never had a value set it's using *zero* space in the buffer.
//! - **Schema Mutations**: The ordering of items in the `columns` property must always remain the same.  It's safe to add new columns to the bottom of the column list or rename columns, but never to remove columns.  Column types cannot be changed safely.  If you need to depreciate a column, set it's name to an empty string. 
//! 
//! Table schemas have a single required property called `columns`.  The `columns` property is an array of arrays that represent all possible columns in the table and their data types.  Any type can be used in columns, including other tables.
//! 
//! Tables do not store the column names in the buffer, only the column index, so this is a very efficient way to store associated data.
//! 
//! If you need flexible column names use a `map` type instead.
//! 
//! ```json
//! {
//!     "type": "table",
//!     "columns": [ // can have between 1 and 255 columns
//!         ["column name",  {"type": "data type for this column"}],
//!         ["name",         {"type": "string"}],
//!         ["tags",         {"type": "list", "of": { // nested list of strings
//!             "type": "string"
//!         }}],
//!         ["age",          {"type": "u8"}], // Uint8 number
//!         ["meta",         {"type": "table", columns: [ // nested table
//!             ["favorite_color",  {"type": "string"}],
//!             ["favorite_sport",  {"type": "string"}]
//!         ]}]
//!     ]
//! }
//! ```
//! 
//! More Details:
//! - [Using NP_Table data type](../collection/table/struct.NP_Table.html)
//! 
//! ## list
//! Lists represent a dynamically sized list of items.  The type for every item in the list is identical and the order of entries is mainted in the buffer.  Lists do not have to contain contiguous entries, gaps can safely and efficiently be stored.
//! 
//! - **Bytewise Sorting**: Unsupported
//! - **Compaction**: Indexes that have had their value cleared will be removed from the buffer.  If a specific index never had a value, it occupies *zero* space.
//! - **Schema Mutations**: None
//! 
//! Lists have a single required property in the schema, `of`.  The `of` property contains another schema for the type of data contained in the list.  Any type is supported, including another list.  Tables cannot have more than 255 columns, and the colum names cannot be longer than 255 UTF8 bytes.
//! 
//! The more items you have in a list, the slower it will be to seek to values towards the end of the list or loop through the list.
//! 
//! ```json
//! // a list of list of strings
//! {
//!     "type": "list",
//!     "of": {
//!         "type": "list",
//!         "of": {"type": "string"}
//!     }
//! }
//! 
//! // list of numbers
//! {
//!     "type": "list",
//!     "of": {"type": "int32"}
//! }
//! ```
//! 
//! More Details:
//! - [Using NP_List data type](../collection/list/struct.NP_List.html)
//! 
//! ## map
//! A map is a dynamically sized list of items where each key is a Vec<u8>.  Every value of a map has the same type.
//! 
//! - **Bytewise Sorting**: Unsupported
//! - **Compaction**: Keys without values are removed from the buffer
//! - **Schema Mutations**: None
//! 
//! Maps have a single required property in the schema, `value`. The property is used to describe the schema of the values for the map.  Keys are always `String`.  Values can be any schema type, including another map.
//! 
//! If you expect to have fixed, predictable keys then use a `table` type instead.  Maps are less efficient than tables because keys are stored in the buffer.  
//! 
//! The more items you have in a map, the slower it will be to seek to values or loop through the map.  
//! 
//! ```json
//! // a map where every value is a string
//! {
//!     "type": "map",
//!     "value": {
//!         "type": "string"
//!     }
//! }
//! ```
//! 
//! More Details:
//! - [Using NP_Map data type](../collection/map/struct.NP_Map.html)
//! 
//! ## tuple
//! A tuple is a fixed size list of items.  Each item has it's own type and index.  Tuples support up to 255 items.
//! 
//! - **Bytewise Sorting**: Supported if all children are scalars that support bytewise sorting and schema `sorted` is set to `true`.
//! - **Compaction**: If `sorted` is true, compaction will not save space.  Otherwise, tuples only reduce in size if children are deleted or children with a dyanmic size are updated.
//! - **Schema Mutations**: If `sorted` is true, none.  Otherwise adding new values to the end of the `values` schema property is safe.
//! 
//! Tuples have a single required property in the schema called `values`.  It's an array of schemas that represnt the tuple values.  Any schema is allowed, including other Tuples.
//! 
//! **Sorting**<br/>
//! You can use tuples to support bytewise sorting across a list of items.  By setting the `sorted` property to `true` you enable a strict mode for the tuple that enables bytewise sorting.  When `sorted` is enabled only scalar values that support sorting are allowed in the schema.  For example, strings/bytes types can only be fixed size.
//! 
//! When `sorted` is true the order of values is gauranteed to be constant across buffers, allowing compound bytewise sorting.
//! 
//! ```json
//! {
//!     "type": "tuple",
//!     "values": [
//!         {"type": "string"},
//!         {"type": "list", "of": {"type": "strings"}},
//!         {"type": "uint64"}
//!     ]
//! }
//! 
//! // tuple for bytewise sorting
//! {
//!     "type": "tuple",
//!     "sorted": true,
//!     "values": [
//!         {"type": "string", "size": 25},
//!         {"type": "uint8"},
//!         {"type": "int64"}
//!     ]
//! }
//! ```
//! 
//! More Details:
//! - [Using NP_Tuple data type](../collection/tuple/struct.NP_Tuple.html) 
//! 
//! 
//! ## string
//! A string is a fixed or dynamically sized collection of utf-8 encoded bytes.
//! 
//! - **Bytewise Sorting**: Supported only if `size` property is set in schema.
//! - **Compaction**: If `size` property is set, compaction cannot reclaim space.  Otherwise it will reclaim space unless all updates have been identical in length.
//! - **Schema Mutations**: If the `size` property is set it's safe to make it smaller, but not larger (this may cause existing string values to truncate, though).  If the field is being used for bytewise sorting, no mutation is safe.
//! 
//!
//! 
//! ```json
//! {
//!     "type": "string"
//! }
//! // fixed size
//! {
//!     "type": "string",
//!     "size": 20
//! }
//! // with default value
//! {
//!     "type": "string",
//!     "default": "Default string value"
//! }
//! ```
//! 
//! More Details:
//! - [Using String data type](../pointer/string/index.html)
//! 
//! ## bytes
//! Bytes are fixed or dynimcally sized Vec<u8> collections. 
//! 
//! - **Bytewise Sorting**: Supported only if `size` property is set in schema.
//! - **Compaction**: If `size` property is set, compaction cannot reclaim space.  Otherwise it will reclaim space unless all updates have been identical in length.
//! - **Schema Mutations**: If the `size` property is set it's safe to make it smaller, but not larger (this may cause existing bytes values to truncate, though).  If the field is being used for bytewise sorting, no mutation is safe.
//! 
//! ```json
//! {
//!     "type": "bytes"
//! }
//! // fixed size
//! {
//!     "type": "bytes",
//!     "size": 20
//! }
//! // with default value
//! {
//!     "type": "bytes",
//!     "default": [1, 2, 3, 4]
//! }
//! ```
//! 
//! More Details:
//! - [Using NP_Bytes data type](../pointer/bytes/struct.NP_Bytes.html)
//! 
//! ## int8, int16, int32, int64
//! Signed integers allow positive or negative whole numbers to be stored.  The bytes are stored in big endian format and converted to unsigned types to allow bytewise sorting.
//! 
//! ```json
//! {
//!     "type": "int8"
//! }
//! // with default value
//! {
//!     "type": "int8",
//!     "default": 20
//! }
//! ```
//! 
//! - **Bytewise Sorting**: Supported
//! - **Compaction**: Updates are done in place, never use additional space.
//! - **Schema Mutations**: None
//! 
//! More Details:
//! - [Using number data types](../pointer/numbers/index.html)
//! 
//! ## uint8, uint16, uint32, uint64
//! Unsgined integers allow only positive whole numbers to be stored.  The bytes are stored in big endian format to allow bytewise sorting.
//! 
//! - **Bytewise Sorting**: Supported
//! - **Compaction**: Updates are done in place, never use additional space.
//! - **Schema Mutations**: None
//! 
//! ```json
//! {
//!     "type": "uint8"
//! }
//! // with default value
//! {
//!     "type": "uint8",
//!     "default": 20
//! }
//! ```
//! 
//! More Details:
//! - [Using number data types](../pointer/numbers/index.html)
//! 
//! ## float, double
//! Allows the storage of floating point numbers of various sizes.  Bytes are stored in big endian format.
//! 
//! - **Bytewise Sorting**: Unsupported, use decimal type.
//! - **Compaction**: Updates are done in place, never use additional space.
//! - **Schema Mutations**: None
//! 
//! ```json
//! {
//!     "type": "float"
//! }
//! // with default value
//! {
//!     "type": "float",
//!     "default": 20.283
//! }
//! ```
//! 
//! More Details:
//! - [Using number data types](../pointer/numbers/index.html)
//! 
//! ## option
//! Allows efficeint storage of a selection between a known collection of ordered strings.  The selection is stored as a single u8 byte, limiting the max number of choices to 255.  Also the choices themselves cannot be longer than 255 UTF8 bytes each.
//! 
//! - **Bytewise Sorting**: Supported
//! - **Compaction**: Updates are done in place, never use additional space.
//! - **Schema Mutations**: You can safely add new choices to the end of the list or update the existing choices in place.  If you need to delete a choice, just make it an empty string.  Changing the order of the choices is destructive as this type only stores the index of the choice it's set to.
//! 
//! There is one required property of this schema called `choices`.  The property should contain an array of strings that represent all possible choices of the option.
//! 
//! ```json
//! {
//!     "type": "option",
//!     "choices": ["choice 1", "choice 2", "etc"]
//! }
//! // with default value
//! {
//!     "type": "option",
//!     "choices": ["choice 1", "choice 2", "etc"],
//!     "default": "etc"
//! }
//! ```
//! 
//! More Details:
//! - [Using NP_Option data type](../pointer/option/struct.NP_Option.html)
//! 
//! ## bool
//! Allows efficent storage of a true or false value.  The value is stored as a single byte that is set to either 1 or 0.
//! 
//! - **Bytewise Sorting**: Supported
//! - **Compaction**: Updates are done in place, never use additional space.
//! - **Schema Mutations**: None
//! 
//! ```json
//! {
//!     "type": "bool"
//! }
//! // with default value
//! {
//!     "type": "bool",
//!     "default": false
//! }
//! ```
//! 
//! More Details:
//! 
//! ## decimal
//! Allows you to store fixed point decimal numbers.  The number of decimal places must be declared in the schema as `exp` property and will be used for every value.
//! 
//! - **Bytewise Sorting**: Supported
//! - **Compaction**: Updates are done in place, never use additional space.
//! - **Schema Mutations**: None
//! 
//! There is a single required property called `exp` that represents the number of decimal points every value will have.
//! 
//! ```json
//! {
//!     "type": "decimal",
//!     "exp": 3
//! }
//! // with default value
//! {
//!     "type": "decimal",
//!     "exp": 3,
//!     "default": 20.293
//! }
//! ```
//! 
//! More Details:
//! - [Using NP_Dec data type](../pointer/dec/struct.NP_Dec.html)
//! 
//! ## geo4, ge8, geo16
//! Allows you to store geographic coordinates with varying levels of accuracy and space usage.  
//! 
//! - **Bytewise Sorting**: Not supported
//! - **Compaction**: Updates are done in place, never use additional space.
//! - **Schema Mutations**: None
//! 
//! Larger geo values take up more space, but allow greater resolution.
//! 
//! | Type  | Bytes | Earth Resolution                       | Decimal Places |
//! |-------|-------|----------------------------------------|----------------|
//! | geo4  | 4     | 1.1km resolution (city)                | 2              |
//! | geo8  | 8     | 11mm resolution (marble)               | 7              |
//! | geo16 | 16    | 110 microns resolution (grain of sand) | 9              |
//! 
//! ```json
//! {
//!     "type": "geo4"
//! }
//! // with default
//! {
//!     "type": "geo4",
//!     "default": {"lat": -20.283, "lng": 19.929}
//! }
//! ```
//! 
//! More Details:
//! - [Using NP_Geo data type](../pointer/geo/struct.NP_Geo.html)
//! 
//! ## ulid
//! Allows you to store a unique ID with a timestamp.  The timestamp is stored in milliseconds since the unix epoch.
//! 
//! - **Bytewise Sorting**: Supported, orders by timestamp. Order is random if timestamp is identical between two values.
//! - **Compaction**: Updates are done in place, never use additional space.
//! - **Schema Mutations**: None
//! 
//! ```json
//! {
//!     "type": "ulid"
//! }
//! // no default supported
//! ```
//! 
//! More Details:
//! - [Using NP_ULID data type](../pointer/ulid/struct.NP_ULID.html)
//! 
//! ## uuid
//! Allows you to store a universally unique ID.
//! 
//! - **Bytewise Sorting**: Supported, but values are random
//! - **Compaction**: Updates are done in place, never use additional space.
//! - **Schema Mutations**: None
//! 
//! ```json
//! {
//!     "type": "uuid"
//! }
//! // no default supported
//! ```
//! 
//! More Details:
//! - [Using NP_UUID data type](../pointer/uuid/struct.NP_UUID.html)
//! 
//! ## date
//! Allows you to store a timestamp as a u64 value.  This is just a thin wrapper around the u64 type.
//! 
//! - **Bytewise Sorting**: Supported
//! - **Compaction**: Updates are done in place, never use additional space.
//! - **Schema Mutations**: None
//! 
//! ```json
//! {
//!     "type": "date"
//! }
//! // with default value (default should be in ms)
//! {
//!     "type": "date",
//!     "default": 1605909163951
//! }
//! ```
//! 
//! More Details:
//! - [Using NP_Date data type](../pointer/date/struct.NP_Date.html)
//!  
//! 
//! ## Next Step
//! 
//! Read about how to initialize a schema into a NoProto Factory.
//! 
//! [Go to NP_Factory docs](../struct.NP_Factory.html)
//! 
use core::{fmt::Debug};
use crate::json_flex::NP_JSON;
use crate::pointer::any::NP_Any;
use crate::pointer::date::NP_Date;
use crate::pointer::uuid::NP_UUID;
use crate::pointer::ulid::NP_ULID;
use crate::pointer::geo::NP_Geo;
use crate::pointer::dec::NP_Dec;
use crate::collection::tuple::NP_Tuple;
use crate::pointer::bytes::NP_Bytes;
use crate::collection::{list::NP_List, table::NP_Table, map::NP_Map};
use crate::pointer::{option::NP_Option, NP_Value};
use crate::error::NP_Error;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::boxed::Box;

/// Simple enum to store the schema types
#[derive(Debug, Clone)]
#[repr(u8)]
#[allow(missing_docs)]
pub enum NP_TypeKeys {
    None = 0,
    Any = 1,
    UTF8String = 2,
    Bytes = 3,
    Int8 = 4,
    Int16 = 5,
    Int32 = 6,
    Int64 = 7,
    Uint8 = 8,
    Uint16 =9,
    Uint32 = 10,
    Uint64 = 11,
    Float = 12,
    Double = 13,
    Decimal = 14,
    Boolean = 15,
    Geo = 16,
    Uuid = 17,
    Ulid = 18,
    Date = 19,
    Enum = 20,
    Table = 21,
    Map = 22, 
    List = 23,
    Tuple = 24
}

impl From<u8> for NP_TypeKeys {
    fn from(value: u8) -> Self {
        if value > 25 { panic!() }
        unsafe { core::mem::transmute(value) }
    }
}

impl NP_TypeKeys {
    /// Convert this NP_TypeKey into a specific type index
    pub fn into_type_idx(&self) -> (u8, String, NP_TypeKeys) {
        match self {
            NP_TypeKeys::None =>       { panic!() }
            NP_TypeKeys::Any =>        {    NP_Any::type_idx() }
            NP_TypeKeys::UTF8String => {    String::type_idx() }
            NP_TypeKeys::Bytes =>      {  NP_Bytes::type_idx() }
            NP_TypeKeys::Int8 =>       {        i8::type_idx() }
            NP_TypeKeys::Int16 =>      {       i16::type_idx() }
            NP_TypeKeys::Int32 =>      {       i32::type_idx() }
            NP_TypeKeys::Int64 =>      {       i64::type_idx() }
            NP_TypeKeys::Uint8 =>      {        u8::type_idx() }
            NP_TypeKeys::Uint16 =>     {       u16::type_idx() }
            NP_TypeKeys::Uint32 =>     {       u32::type_idx() }
            NP_TypeKeys::Uint64 =>     {       u64::type_idx() }
            NP_TypeKeys::Float =>      {       f32::type_idx() }
            NP_TypeKeys::Double =>     {       f64::type_idx() }
            NP_TypeKeys::Decimal =>    {    NP_Dec::type_idx() }
            NP_TypeKeys::Boolean =>    {      bool::type_idx() }
            NP_TypeKeys::Geo =>        {    NP_Geo::type_idx() }
            NP_TypeKeys::Uuid =>       {   NP_UUID::type_idx() }
            NP_TypeKeys::Ulid =>       {   NP_ULID::type_idx() }
            NP_TypeKeys::Date =>       {   NP_Date::type_idx() }
            NP_TypeKeys::Enum =>       { NP_Option::type_idx() }
            NP_TypeKeys::Table =>      {  NP_Table::type_idx() }
            NP_TypeKeys::Map =>        {    NP_Map::type_idx() }
            NP_TypeKeys::List =>       {   NP_List::type_idx() }
            NP_TypeKeys::Tuple =>      {  NP_Tuple::type_idx() }
        }
    }
}

/// When a schema is parsed from JSON or Bytes, it is stored in this recursive type
/// 
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum NP_Parsed_Schema {
    None,
    Any        { sortable: bool, i:NP_TypeKeys },
    UTF8String { sortable: bool, i:NP_TypeKeys, default: Option<Box<String>>, size: u16 },
    Bytes      { sortable: bool, i:NP_TypeKeys, default: Option<Box<Vec<u8>>>, size: u16 },
    Int8       { sortable: bool, i:NP_TypeKeys, default: Option<Box<i8>> },
    Int16      { sortable: bool, i:NP_TypeKeys, default: Option<Box<i16>> },
    Int32      { sortable: bool, i:NP_TypeKeys, default: Option<Box<i32>> },
    Int64      { sortable: bool, i:NP_TypeKeys, default: Option<Box<i64>> },
    Uint8      { sortable: bool, i:NP_TypeKeys, default: Option<Box<u8>> },
    Uint16     { sortable: bool, i:NP_TypeKeys, default: Option<Box<u16>> },
    Uint32     { sortable: bool, i:NP_TypeKeys, default: Option<Box<u32>> },
    Uint64     { sortable: bool, i:NP_TypeKeys, default: Option<Box<u64>> },
    Float      { sortable: bool, i:NP_TypeKeys, default: Option<Box<f32>> },
    Double     { sortable: bool, i:NP_TypeKeys, default: Option<Box<f64>> },
    Decimal    { sortable: bool, i:NP_TypeKeys, default: Option<Box<NP_Dec>>, exp: u8 },
    Boolean    { sortable: bool, i:NP_TypeKeys, default: Option<Box<bool>> },
    Geo        { sortable: bool, i:NP_TypeKeys, default: Option<Box<NP_Geo>>, size: u8 },
    Date       { sortable: bool, i:NP_TypeKeys, default: Option<Box<NP_Date>> },
    Enum       { sortable: bool, i:NP_TypeKeys, default: Option<Box<u8>>, choices: Vec<String> },
    Uuid       { sortable: bool, i:NP_TypeKeys },
    Ulid       { sortable: bool, i:NP_TypeKeys },
    Table      { sortable: bool, i:NP_TypeKeys, columns: Vec<(u8, String, Box<NP_Parsed_Schema>)> },
    Map        { sortable: bool, i:NP_TypeKeys, value: Box<NP_Parsed_Schema>}, 
    List       { sortable: bool, i:NP_TypeKeys, of: Box<NP_Parsed_Schema> },
    Tuple      { sortable: bool, i:NP_TypeKeys, values: Vec<Box<NP_Parsed_Schema>>}
}


impl NP_Parsed_Schema {

    /// Get the type key for this schema
    pub fn get_type_key(&self) -> NP_TypeKeys {
        self.into_type_data().2
    }

    /// Get the type data fo a given schema value
    pub fn into_type_data(&self) -> (u8, String, NP_TypeKeys) {
        match self {
            NP_Parsed_Schema::None => (0, String::from(""), NP_TypeKeys::None),
            NP_Parsed_Schema::Any        { sortable: _, i }                        => { i.into_type_idx() }
            NP_Parsed_Schema::UTF8String { sortable: _, i, size:_, default:_ }     => { i.into_type_idx() }
            NP_Parsed_Schema::Bytes      { sortable: _, i, size:_, default:_ }     => { i.into_type_idx() }
            NP_Parsed_Schema::Int8       { sortable: _, i, default: _ }            => { i.into_type_idx() }
            NP_Parsed_Schema::Int16      { sortable: _, i , default: _ }           => { i.into_type_idx() }
            NP_Parsed_Schema::Int32      { sortable: _, i , default: _ }           => { i.into_type_idx() }
            NP_Parsed_Schema::Int64      { sortable: _, i , default: _ }           => { i.into_type_idx() }
            NP_Parsed_Schema::Uint8      { sortable: _, i , default: _ }           => { i.into_type_idx() }
            NP_Parsed_Schema::Uint16     { sortable: _, i , default: _ }           => { i.into_type_idx() }
            NP_Parsed_Schema::Uint32     { sortable: _, i , default: _ }           => { i.into_type_idx() }
            NP_Parsed_Schema::Uint64     { sortable: _, i , default: _ }           => { i.into_type_idx() }
            NP_Parsed_Schema::Float      { sortable: _, i , default: _ }           => { i.into_type_idx() }
            NP_Parsed_Schema::Double     { sortable: _, i , default: _ }           => { i.into_type_idx() }
            NP_Parsed_Schema::Decimal    { sortable: _, i, exp:_, default:_ }      => { i.into_type_idx() }
            NP_Parsed_Schema::Boolean    { sortable: _, i, default:_ }             => { i.into_type_idx() }
            NP_Parsed_Schema::Geo        { sortable: _, i, default:_, size:_ }     => { i.into_type_idx() }
            NP_Parsed_Schema::Uuid       { sortable: _, i }                        => { i.into_type_idx() }
            NP_Parsed_Schema::Ulid       { sortable: _, i }                        => { i.into_type_idx() }
            NP_Parsed_Schema::Date       { sortable: _, i, default:_ }             => { i.into_type_idx() }
            NP_Parsed_Schema::Enum       { sortable: _, i, default:_, choices: _ } => { i.into_type_idx() }
            NP_Parsed_Schema::Table      { sortable: _, i, columns:_ }             => { i.into_type_idx() }
            NP_Parsed_Schema::Map        { sortable: _, i, value:_ }               => { i.into_type_idx() }
            NP_Parsed_Schema::List       { sortable: _, i, of:_ }                  => { i.into_type_idx() }
            NP_Parsed_Schema::Tuple      { sortable: _, i, values:_ }              => { i.into_type_idx() }
        }
    }

    /// Return if this schema is sortable
    pub fn is_sortable(&self) -> bool {
        match self {
            NP_Parsed_Schema::None => false,
            NP_Parsed_Schema::Any        { sortable, i: _ }                        => { *sortable }
            NP_Parsed_Schema::UTF8String { sortable, i: _, size:_, default:_ }     => { *sortable }
            NP_Parsed_Schema::Bytes      { sortable, i: _, size:_, default:_ }     => { *sortable }
            NP_Parsed_Schema::Int8       { sortable, i: _, default: _ }            => { *sortable }
            NP_Parsed_Schema::Int16      { sortable, i: _ , default: _ }           => { *sortable }
            NP_Parsed_Schema::Int32      { sortable, i: _ , default: _ }           => { *sortable }
            NP_Parsed_Schema::Int64      { sortable, i: _ , default: _ }           => { *sortable }
            NP_Parsed_Schema::Uint8      { sortable, i: _ , default: _ }           => { *sortable }
            NP_Parsed_Schema::Uint16     { sortable, i: _ , default: _ }           => { *sortable }
            NP_Parsed_Schema::Uint32     { sortable, i: _ , default: _ }           => { *sortable }
            NP_Parsed_Schema::Uint64     { sortable, i: _ , default: _ }           => { *sortable }
            NP_Parsed_Schema::Float      { sortable, i: _ , default: _ }           => { *sortable }
            NP_Parsed_Schema::Double     { sortable, i: _ , default: _ }           => { *sortable }
            NP_Parsed_Schema::Decimal    { sortable, i: _, exp:_, default:_ }      => { *sortable }
            NP_Parsed_Schema::Boolean    { sortable, i: _, default:_ }             => { *sortable }
            NP_Parsed_Schema::Geo        { sortable, i: _, default:_, size:_ }     => { *sortable }
            NP_Parsed_Schema::Uuid       { sortable, i: _ }                        => { *sortable }
            NP_Parsed_Schema::Ulid       { sortable, i: _ }                        => { *sortable }
            NP_Parsed_Schema::Date       { sortable, i: _, default:_ }             => { *sortable }
            NP_Parsed_Schema::Enum       { sortable, i: _, default:_, choices: _ } => { *sortable }
            NP_Parsed_Schema::Table      { sortable, i: _, columns:_ }             => { *sortable }
            NP_Parsed_Schema::Map        { sortable, i: _, value:_ }               => { *sortable }
            NP_Parsed_Schema::List       { sortable, i: _, of:_ }                  => { *sortable }
            NP_Parsed_Schema::Tuple      { sortable, i: _, values:_ }              => { *sortable }
        }
    }
}

/// New NP Schema
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct NP_Schema {
    /// is this schema sortable?
    pub is_sortable: bool,
    /// schema bytes
    pub bytes: Vec<u8>,
    /// recursive parsed schema
    pub parsed: Box<NP_Parsed_Schema>
}

macro_rules! schema_check {
    ($t: ty, $json: expr) => {
        match <$t>::from_json_to_schema($json)? {
            Some(x) => return Ok(x), None => {}
        }
    }
}

impl NP_Schema {

    /// Get a JSON represenatation of this schema
    pub fn to_json(&self) -> Result<NP_JSON, NP_Error> {
        NP_Schema::_type_to_json(&self.parsed)
    }

    /// Recursive function parse schema into JSON
    #[doc(hidden)]
    pub fn _type_to_json(parsed_schema: &Box<NP_Parsed_Schema>) -> Result<NP_JSON, NP_Error> {
        match **parsed_schema {
            NP_Parsed_Schema::Any        { sortable: _, i:_ }                         => {    NP_Any::schema_to_json(parsed_schema) }
            NP_Parsed_Schema::UTF8String { sortable: _, i:_, size:_, default:_ }      => {    String::schema_to_json(parsed_schema) }
            NP_Parsed_Schema::Bytes      { sortable: _, i:_, size:_, default:_ }      => {  NP_Bytes::schema_to_json(parsed_schema) }
            NP_Parsed_Schema::Int8       { sortable: _, i:_, default: _ }             => {        i8::schema_to_json(parsed_schema) }
            NP_Parsed_Schema::Int16      { sortable: _, i:_ , default: _ }            => {       i16::schema_to_json(parsed_schema) }
            NP_Parsed_Schema::Int32      { sortable: _, i:_ , default: _ }            => {       i32::schema_to_json(parsed_schema) }
            NP_Parsed_Schema::Int64      { sortable: _, i:_ , default: _ }            => {       i64::schema_to_json(parsed_schema) }
            NP_Parsed_Schema::Uint8      { sortable: _, i:_ , default: _ }            => {        u8::schema_to_json(parsed_schema) }
            NP_Parsed_Schema::Uint16     { sortable: _, i:_ , default: _ }            => {       u16::schema_to_json(parsed_schema) }
            NP_Parsed_Schema::Uint32     { sortable: _, i:_ , default: _ }            => {       u32::schema_to_json(parsed_schema) }
            NP_Parsed_Schema::Uint64     { sortable: _, i:_ , default: _ }            => {       u64::schema_to_json(parsed_schema) }
            NP_Parsed_Schema::Float      { sortable: _, i:_ , default: _ }            => {       f32::schema_to_json(parsed_schema) }
            NP_Parsed_Schema::Double     { sortable: _, i:_ , default: _ }            => {       f64::schema_to_json(parsed_schema) }
            NP_Parsed_Schema::Decimal    { sortable: _, i:_, exp:_, default:_ }       => {    NP_Dec::schema_to_json(parsed_schema) }
            NP_Parsed_Schema::Boolean    { sortable: _, i:_, default:_ }              => {      bool::schema_to_json(parsed_schema) }
            NP_Parsed_Schema::Geo        { sortable: _, i:_, default:_, size:_ }      => {    NP_Geo::schema_to_json(parsed_schema) }
            NP_Parsed_Schema::Uuid       { sortable: _, i:_ }                         => {   NP_UUID::schema_to_json(parsed_schema) }
            NP_Parsed_Schema::Ulid       { sortable: _, i:_ }                         => {   NP_ULID::schema_to_json(parsed_schema) }
            NP_Parsed_Schema::Date       { sortable: _, i:_, default:_ }              => {   NP_Date::schema_to_json(parsed_schema) }
            NP_Parsed_Schema::Enum       { sortable: _, i:_, default:_, choices: _ }  => { NP_Option::schema_to_json(parsed_schema) }
            NP_Parsed_Schema::Table      { sortable: _, i:_, columns:_ }              => {  NP_Table::schema_to_json(parsed_schema) }
            NP_Parsed_Schema::Map        { sortable: _, i:_, value:_ }                => {    NP_Map::schema_to_json(parsed_schema) }
            NP_Parsed_Schema::List       { sortable: _, i:_, of:_ }                   => {   NP_List::schema_to_json(parsed_schema) }
            NP_Parsed_Schema::Tuple      { sortable: _, i:_, values:_ }               => {  NP_Tuple::schema_to_json(parsed_schema) }
            _ => { panic!() }
        }
    }

    /// Get type string for this schema
    #[doc(hidden)]
    pub fn _get_type(json_schema: &NP_JSON) -> Result<String, NP_Error> {
        match &json_schema["type"] {
            NP_JSON::String(x) => {
                Ok(x.clone())
            },
            _ => {
                Err(NP_Error::new("Schemas must have a 'type' property!"))
            }
        }
    }

    /// Parse a schema out of schema bytes
    pub fn from_bytes(address: usize, bytes: &Vec<u8>) -> NP_Parsed_Schema {
        let this_type = NP_TypeKeys::from(bytes[address]);
        match this_type {
            NP_TypeKeys::None =>       { panic!() }
            NP_TypeKeys::Any =>        {    NP_Any::from_bytes_to_schema(address, bytes) }
            NP_TypeKeys::UTF8String => {    String::from_bytes_to_schema(address, bytes) }
            NP_TypeKeys::Bytes =>      {  NP_Bytes::from_bytes_to_schema(address, bytes) }
            NP_TypeKeys::Int8 =>       {        i8::from_bytes_to_schema(address, bytes) }
            NP_TypeKeys::Int16 =>      {       i16::from_bytes_to_schema(address, bytes) }
            NP_TypeKeys::Int32 =>      {       i32::from_bytes_to_schema(address, bytes) }
            NP_TypeKeys::Int64 =>      {       i64::from_bytes_to_schema(address, bytes) }
            NP_TypeKeys::Uint8 =>      {        u8::from_bytes_to_schema(address, bytes) }
            NP_TypeKeys::Uint16 =>     {       u16::from_bytes_to_schema(address, bytes) }
            NP_TypeKeys::Uint32 =>     {       u32::from_bytes_to_schema(address, bytes) }
            NP_TypeKeys::Uint64 =>     {       u64::from_bytes_to_schema(address, bytes) }
            NP_TypeKeys::Float =>      {       f32::from_bytes_to_schema(address, bytes) }
            NP_TypeKeys::Double =>     {       f64::from_bytes_to_schema(address, bytes) }
            NP_TypeKeys::Decimal =>    {    NP_Dec::from_bytes_to_schema(address, bytes) }
            NP_TypeKeys::Boolean =>    {      bool::from_bytes_to_schema(address, bytes) }
            NP_TypeKeys::Geo =>        {    NP_Geo::from_bytes_to_schema(address, bytes) }
            NP_TypeKeys::Uuid =>       {   NP_UUID::from_bytes_to_schema(address, bytes) }
            NP_TypeKeys::Ulid =>       {   NP_ULID::from_bytes_to_schema(address, bytes) }
            NP_TypeKeys::Date =>       {   NP_Date::from_bytes_to_schema(address, bytes) }
            NP_TypeKeys::Enum =>       { NP_Option::from_bytes_to_schema(address, bytes) }
            NP_TypeKeys::Table =>      {  NP_Table::from_bytes_to_schema(address, bytes) }
            NP_TypeKeys::Map =>        {    NP_Map::from_bytes_to_schema(address, bytes) }
            NP_TypeKeys::List =>       {   NP_List::from_bytes_to_schema(address, bytes) }
            NP_TypeKeys::Tuple =>      {  NP_Tuple::from_bytes_to_schema(address, bytes) }
        }
    }

    /// Parse schema from JSON object
    /// 
    /// Given a valid JSON schema, parse and validate, then provide a compiled byte schema.
    /// 
    /// If you need a quick way to convert JSON to schema bytes without firing up an NP_Factory, this will do the trick.
    /// 
    pub fn from_json(json_schema: Box<NP_JSON>) -> Result<(Vec<u8>, NP_Parsed_Schema), NP_Error> {

        schema_check!(NP_Any,          &json_schema);
        schema_check!(String,          &json_schema);
        schema_check!(NP_Bytes,        &json_schema);

        schema_check!(i8,              &json_schema);
        schema_check!(i16,             &json_schema);
        schema_check!(i32,             &json_schema);
        schema_check!(i64,             &json_schema);

        schema_check!(u8,              &json_schema);
        schema_check!(u16,             &json_schema);
        schema_check!(u32,             &json_schema);
        schema_check!(u64,             &json_schema);
        
        schema_check!(f32,             &json_schema);
        schema_check!(f64,             &json_schema);

        schema_check!(NP_Dec,          &json_schema);
        schema_check!(bool,            &json_schema);
        schema_check!(NP_Geo,          &json_schema);
        schema_check!(NP_ULID,         &json_schema);
        schema_check!(NP_UUID,         &json_schema);
        schema_check!(NP_Date,         &json_schema);
        schema_check!(NP_Option,       &json_schema);

        schema_check!(NP_Table,        &json_schema);
        schema_check!(NP_Map,          &json_schema);
        schema_check!(NP_List,         &json_schema);
        schema_check!(NP_Tuple,        &json_schema);

        let mut err_msg = String::from("Can't find a type that matches this schema! ");
        err_msg.push_str(json_schema.stringify().as_str());
        Err(NP_Error::new(err_msg.as_str()))
    }
}
