// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! Parquet schema parser.
//! Provides methods to parse and validate string message type into Parquet
//! [`ParquetType`](crate::schema::types::ParquetType).
//!
//! # Example
//!
//! ```rust
//! use schema::parser::parse_message_type;
//!
//! let message_type = "
//!   message spark_schema {
//!     OPTIONAL BYTE_ARRAY a (UTF8);
//!     REQUIRED INT32 b;
//!     REQUIRED DOUBLE c;
//!     REQUIRED BOOLEAN d;
//!     OPTIONAL group e (LIST) {
//!       REPEATED group list {
//!         REQUIRED INT32 element;
//!       }
//!     }
//!   }
//! ";
//!
//! let schema = parse_message_type(message_type).expect("Expected valid schema");
//! println!("{:?}", schema);
//! ```

use super::types::{converted_to_primitive_converted, GroupConvertedType, PrimitiveConvertedType};
use super::types::{ParquetType, PhysicalType};
use super::{
    BsonType, DateType, EnumType, IntType, JsonType, ListType, LogicalType, MapType, MicroSeconds,
    MilliSeconds, NullType, Repetition, StringType, TimeType, TimeUnit, TimestampType, Type,
    UUIDType,
};
use crate::errors::{ParquetError, Result};

fn converted_group_from_str(s: &str) -> Result<GroupConvertedType> {
    use GroupConvertedType::*;
    Ok(match s {
        "MAP" => Map,
        "MAP_KEY_VALUE" => MapKeyValue,
        "LIST" => List,
        other => return Err(general_err!("Invalid converted type {}", other)),
    })
}

fn converted_primitive_from_str(s: &str) -> Result<parquet_format::ConvertedType> {
    use parquet_format::ConvertedType::*;
    Ok(match s {
        "UTF8" => Utf8,
        "ENUM" => Enum,
        "DECIMAL" => Decimal,
        "DATE" => Date,
        "TIME_MILLIS" => TimeMillis,
        "TIME_MICROS" => TimeMicros,
        "TIMESTAMP_MILLIS" => TimestampMillis,
        "TIMESTAMP_MICROS" => TimestampMicros,
        "UINT_8" => Uint8,
        "UINT_16" => Uint16,
        "UINT_32" => Uint32,
        "UINT_64" => Uint64,
        "INT_8" => Int8,
        "INT_16" => Int16,
        "INT_32" => Int32,
        "INT_64" => Int64,
        "JSON" => Json,
        "BSON" => Bson,
        "INTERVAL" => Interval,
        other => return Err(general_err!("Invalid converted type {}", other)),
    })
}

fn logical_from_str(s: &str) -> Result<LogicalType> {
    match s {
        "INTEGER(8,true)" => Ok(LogicalType::INTEGER(IntType {
            bit_width: 8,
            is_signed: true,
        })),
        "INTEGER(16,true)" => Ok(LogicalType::INTEGER(IntType {
            bit_width: 16,
            is_signed: true,
        })),
        "INTEGER(32,true)" => Ok(LogicalType::INTEGER(IntType {
            bit_width: 32,
            is_signed: true,
        })),
        "INTEGER(64,true)" => Ok(LogicalType::INTEGER(IntType {
            bit_width: 64,
            is_signed: true,
        })),
        "INTEGER(8,false)" => Ok(LogicalType::INTEGER(IntType {
            bit_width: 8,
            is_signed: false,
        })),
        "INTEGER(16,false)" => Ok(LogicalType::INTEGER(IntType {
            bit_width: 16,
            is_signed: false,
        })),
        "INTEGER(32,false)" => Ok(LogicalType::INTEGER(IntType {
            bit_width: 32,
            is_signed: false,
        })),
        "INTEGER(64,false)" => Ok(LogicalType::INTEGER(IntType {
            bit_width: 64,
            is_signed: false,
        })),
        "MAP" => Ok(LogicalType::MAP(MapType {})),
        "LIST" => Ok(LogicalType::LIST(ListType {})),
        "ENUM" => Ok(LogicalType::ENUM(EnumType {})),
        // TODO: ARROW-11365
        // "DECIMAL" => Ok(LogicalType::DECIMAL),
        "DATE" => Ok(LogicalType::DATE(DateType {})),
        "TIME(MILLIS,true)" => Ok(LogicalType::TIME(TimeType {
            is_adjusted_to_u_t_c: true,
            unit: TimeUnit::MILLIS(MilliSeconds {}),
        })),
        "TIME(MILLIS,false)" => Ok(LogicalType::TIME(TimeType {
            is_adjusted_to_u_t_c: false,
            unit: TimeUnit::MILLIS(MilliSeconds {}),
        })),
        "TIME(MICROS,true)" => Ok(LogicalType::TIME(TimeType {
            is_adjusted_to_u_t_c: true,
            unit: TimeUnit::MICROS(MicroSeconds {}),
        })),
        "TIME(MICROS,false)" => Ok(LogicalType::TIME(TimeType {
            is_adjusted_to_u_t_c: false,
            unit: TimeUnit::MICROS(MicroSeconds {}),
        })),
        "TIMESTAMP(MILLIS,true)" => Ok(LogicalType::TIMESTAMP(TimestampType {
            is_adjusted_to_u_t_c: true,
            unit: TimeUnit::MILLIS(MilliSeconds {}),
        })),
        "TIMESTAMP(MILLIS,false)" => Ok(LogicalType::TIMESTAMP(TimestampType {
            is_adjusted_to_u_t_c: false,
            unit: TimeUnit::MILLIS(MilliSeconds {}),
        })),
        "TIMESTAMP(MICROS,true)" => Ok(LogicalType::TIMESTAMP(TimestampType {
            is_adjusted_to_u_t_c: true,
            unit: TimeUnit::MICROS(MicroSeconds {}),
        })),
        "TIMESTAMP(MICROS,false)" => Ok(LogicalType::TIMESTAMP(TimestampType {
            is_adjusted_to_u_t_c: false,
            unit: TimeUnit::MICROS(MicroSeconds {}),
        })),
        "TIMESTAMP(NANOS,true)" => Ok(LogicalType::TIMESTAMP(TimestampType {
            is_adjusted_to_u_t_c: true,
            unit: TimeUnit::MICROS(MicroSeconds {}),
        })),
        "TIMESTAMP(NANOS,false)" => Ok(LogicalType::TIMESTAMP(TimestampType {
            is_adjusted_to_u_t_c: false,
            unit: TimeUnit::MICROS(MicroSeconds {}),
        })),
        "STRING" => Ok(LogicalType::STRING(StringType {})),
        "JSON" => Ok(LogicalType::JSON(JsonType {})),
        "BSON" => Ok(LogicalType::BSON(BsonType {})),
        "UUID" => Ok(LogicalType::UUID(UUIDType {})),
        "UNKNOWN" => Ok(LogicalType::UNKNOWN(NullType {})),
        "INTERVAL" => Err(general_err!("Interval logical type not yet supported")),
        other => Err(general_err!("Invalid logical type {}", other)),
    }
}

fn repetition_from_str(s: &str) -> Result<Repetition> {
    Ok(match s {
        "REQUIRED" => Repetition::Required,
        "OPTIONAL" => Repetition::Optional,
        "REPEATED" => Repetition::Repeated,
        other => return Err(general_err!("Invalid repetition {}", other)),
    })
}

fn type_from_str(s: &str) -> Result<Type> {
    match s {
        "BOOLEAN" => Ok(Type::Boolean),
        "INT32" => Ok(Type::Int32),
        "INT64" => Ok(Type::Int64),
        "INT96" => Ok(Type::Int96),
        "FLOAT" => Ok(Type::Float),
        "DOUBLE" => Ok(Type::Double),
        "BYTE_ARRAY" | "BINARY" => Ok(Type::ByteArray),
        "FIXED_LEN_BYTE_ARRAY" => Ok(Type::FixedLenByteArray),
        other => Err(general_err!("Invalid type {}", other)),
    }
}

/// Parses message type as string into a Parquet [`ParquetType`](crate::schema::types::ParquetType)
/// which, for example, could be used to extract individual columns. Returns Parquet
/// general error when parsing or validation fails.
pub fn parse_message_type(message_type: &str) -> Result<ParquetType> {
    let mut parser = Parser {
        tokenizer: &mut Tokenizer::from_str(message_type),
    };
    parser.parse_message_type()
}

/// Tokenizer to split message type string into tokens that are separated using characters
/// defined in `is_schema_delim` method. Tokenizer also preserves delimiters as tokens.
/// Tokenizer provides Iterator interface to process tokens; it also allows to step back
/// to reprocess previous tokens.
struct Tokenizer<'a> {
    // List of all tokens for a string
    tokens: Vec<&'a str>,
    // Current index of vector
    index: usize,
}

impl<'a> Tokenizer<'a> {
    // Create tokenizer from message type string
    pub fn from_str(string: &'a str) -> Self {
        let vec = string
            .split_whitespace()
            .flat_map(|t| Self::split_token(t))
            .collect();
        Tokenizer {
            tokens: vec,
            index: 0,
        }
    }

    // List of all special characters in schema
    fn is_schema_delim(c: char) -> bool {
        c == ';' || c == '{' || c == '}' || c == '(' || c == ')' || c == '=' || c == ','
    }

    /// Splits string into tokens; input string can already be token or can contain
    /// delimiters, e.g. required" -> Vec("required") and
    /// "(UTF8);" -> Vec("(", "UTF8", ")", ";")
    fn split_token(string: &str) -> Vec<&str> {
        let mut buffer: Vec<&str> = Vec::new();
        let mut tail = string;
        while let Some(index) = tail.find(Self::is_schema_delim) {
            let (h, t) = tail.split_at(index);
            if !h.is_empty() {
                buffer.push(h);
            }
            buffer.push(&t[0..1]);
            tail = &t[1..];
        }
        if !tail.is_empty() {
            buffer.push(tail);
        }
        buffer
    }

    // Move pointer to a previous element
    fn backtrack(&mut self) {
        self.index -= 1;
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        if self.index < self.tokens.len() {
            self.index += 1;
            Some(self.tokens[self.index - 1])
        } else {
            None
        }
    }
}

/// Internal Schema parser.
/// Traverses message type using tokenizer and parses each group/primitive type
/// recursively.
struct Parser<'a> {
    tokenizer: &'a mut Tokenizer<'a>,
}

// Utility function to assert token on validity.
fn assert_token(token: Option<&str>, expected: &str) -> Result<()> {
    match token {
        Some(value) if value == expected => Ok(()),
        Some(other) => Err(general_err!(
            "Expected '{}', found token '{}'",
            expected,
            other
        )),
        None => Err(general_err!(
            "Expected '{}', but no token found (None)",
            expected
        )),
    }
}

// Utility function to parse i32 or return general error.
fn parse_i32(value: Option<&str>, not_found_msg: &str, parse_fail_msg: &str) -> Result<i32> {
    value
        .ok_or_else(|| general_err!(not_found_msg))
        .and_then(|v| v.parse::<i32>().map_err(|_| general_err!(parse_fail_msg)))
}

impl<'a> Parser<'a> {
    // Entry function to parse message type, uses internal tokenizer.
    fn parse_message_type(&mut self) -> Result<ParquetType> {
        // Check that message type starts with "message".
        match self.tokenizer.next() {
            Some("message") => {
                let name = self
                    .tokenizer
                    .next()
                    .ok_or_else(|| general_err!("Expected name, found None"))?;
                let fields = self.parse_child_types()?;
                Ok(ParquetType::from_fields(name.to_string(), fields))
            }
            _ => Err(general_err!("Message type does not start with 'message'")),
        }
    }

    // Parses child types for a current group type.
    // This is only invoked on root and group types.
    fn parse_child_types(&mut self) -> Result<Vec<ParquetType>> {
        assert_token(self.tokenizer.next(), "{")?;
        let mut vec = Vec::new();
        while let Some(value) = self.tokenizer.next() {
            if value == "}" {
                break;
            } else {
                self.tokenizer.backtrack();
                vec.push(self.add_type()?);
            }
        }
        Ok(vec)
    }

    fn add_type(&mut self) -> Result<ParquetType> {
        // Parse repetition
        let repetition = self
            .tokenizer
            .next()
            .ok_or_else(|| general_err!("Expected repetition, found None"))
            .and_then(|v| repetition_from_str(&v.to_uppercase()))?;

        match self.tokenizer.next() {
            Some(group) if group.to_uppercase() == "GROUP" => self.add_group_type(Some(repetition)),
            Some(type_string) => {
                let physical_type = type_from_str(&type_string.to_uppercase())?;
                self.add_primitive_type(repetition, physical_type)
            }
            None => Err(general_err!("Invalid type, could not extract next token")),
        }
    }

    fn add_group_type(&mut self, repetition: Option<Repetition>) -> Result<ParquetType> {
        // Parse name of the group type
        let name = self
            .tokenizer
            .next()
            .ok_or_else(|| general_err!("Expected name, found None"))?;

        // Parse converted type if exists
        let converted_type = if let Some("(") = self.tokenizer.next() {
            let tpe = self
                .tokenizer
                .next()
                .ok_or_else(|| general_err!("Expected converted type, found None"))
                .and_then(|v| converted_group_from_str(&v.to_uppercase()))?;
            assert_token(self.tokenizer.next(), ")")?;
            Some(tpe)
        } else {
            self.tokenizer.backtrack();
            None
        };

        // Parse optional id
        let id = if let Some("=") = self.tokenizer.next() {
            self.tokenizer.next().and_then(|v| v.parse::<i32>().ok())
        } else {
            self.tokenizer.backtrack();
            None
        };

        let fields = self.parse_child_types()?;

        Ok(ParquetType::from_converted(
            name.to_string(),
            fields,
            repetition,
            converted_type,
            id,
        ))
    }

    fn add_primitive_type(
        &mut self,
        repetition: Repetition,
        physical_type: Type,
    ) -> Result<ParquetType> {
        // Read type length if the type is FIXED_LEN_BYTE_ARRAY.
        let mut length: i32 = -1;
        if physical_type == Type::FixedLenByteArray {
            assert_token(self.tokenizer.next(), "(")?;
            length = parse_i32(
                self.tokenizer.next(),
                "Expected length for FIXED_LEN_BYTE_ARRAY, found None",
                "Failed to parse length for FIXED_LEN_BYTE_ARRAY",
            )?;
            assert_token(self.tokenizer.next(), ")")?;
        }

        // Parse name of the primitive type
        let name = self
            .tokenizer
            .next()
            .ok_or_else(|| general_err!("Expected name, found None"))?;

        // Parse converted type
        let converted_type = if let Some("(") = self.tokenizer.next() {
            let tpe = self
                .tokenizer
                .next()
                .ok_or_else(|| general_err!("Expected converted type, found None"))
                .and_then(|v| converted_primitive_from_str(&v.to_uppercase()))?;

            let tpe = match tpe {
                parquet_format::ConvertedType::Decimal => {
                    assert_token(self.tokenizer.next(), "(")?;
                    // Parse precision
                    let precision = parse_i32(
                        self.tokenizer.next(),
                        "Expected precision, found None",
                        "Failed to parse precision for DECIMAL type",
                    )?;

                    // Parse scale
                    let scale = if let Some(",") = self.tokenizer.next() {
                        parse_i32(
                            self.tokenizer.next(),
                            "Expected scale, found None",
                            "Failed to parse scale for DECIMAL type",
                        )?
                    } else {
                        // Scale is not provided, set it to 0.
                        self.tokenizer.backtrack();
                        0
                    };

                    assert_token(self.tokenizer.next(), ")")?;
                    PrimitiveConvertedType::Decimal(precision, scale)
                }
                _ => converted_to_primitive_converted(&tpe)?,
            };

            assert_token(self.tokenizer.next(), ")")?;
            Some(tpe)
        } else {
            self.tokenizer.backtrack();
            None
        };

        // Parse optional id
        let id = if let Some("=") = self.tokenizer.next() {
            self.tokenizer.next().and_then(|v| v.parse::<i32>().ok())
        } else {
            self.tokenizer.backtrack();
            None
        };
        assert_token(self.tokenizer.next(), ";")?;

        let physical_type = match physical_type {
            Type::Boolean => PhysicalType::Boolean,
            Type::Int32 => PhysicalType::Int32,
            Type::Int64 => PhysicalType::Int64,
            Type::Int96 => PhysicalType::Int96,
            Type::Float => PhysicalType::Float,
            Type::Double => PhysicalType::Double,
            Type::ByteArray => PhysicalType::ByteArray,
            Type::FixedLenByteArray => PhysicalType::FixedLenByteArray(length),
        };

        ParquetType::try_from_primitive(
            name.to_string(),
            physical_type,
            repetition,
            converted_type,
            id,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_empty_string() {
        assert_eq!(Tokenizer::from_str("").next(), None);
    }

    #[test]
    fn test_tokenize_delimiters() {
        let mut iter = Tokenizer::from_str(",;{}()=");
        assert_eq!(iter.next(), Some(","));
        assert_eq!(iter.next(), Some(";"));
        assert_eq!(iter.next(), Some("{"));
        assert_eq!(iter.next(), Some("}"));
        assert_eq!(iter.next(), Some("("));
        assert_eq!(iter.next(), Some(")"));
        assert_eq!(iter.next(), Some("="));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_tokenize_delimiters_with_whitespaces() {
        let mut iter = Tokenizer::from_str(" , ; { } ( ) = ");
        assert_eq!(iter.next(), Some(","));
        assert_eq!(iter.next(), Some(";"));
        assert_eq!(iter.next(), Some("{"));
        assert_eq!(iter.next(), Some("}"));
        assert_eq!(iter.next(), Some("("));
        assert_eq!(iter.next(), Some(")"));
        assert_eq!(iter.next(), Some("="));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_tokenize_words() {
        let mut iter = Tokenizer::from_str("abc def ghi jkl mno");
        assert_eq!(iter.next(), Some("abc"));
        assert_eq!(iter.next(), Some("def"));
        assert_eq!(iter.next(), Some("ghi"));
        assert_eq!(iter.next(), Some("jkl"));
        assert_eq!(iter.next(), Some("mno"));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_tokenize_backtrack() {
        let mut iter = Tokenizer::from_str("abc;");
        assert_eq!(iter.next(), Some("abc"));
        assert_eq!(iter.next(), Some(";"));
        iter.backtrack();
        assert_eq!(iter.next(), Some(";"));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_tokenize_message_type() {
        let schema = "
    message schema {
      required int32 a;
      optional binary c (UTF8);
      required group d {
        required int32 a;
        optional binary c (UTF8);
      }
      required group e (LIST) {
        repeated group list {
          required int32 element;
        }
      }
    }
    ";
        let iter = Tokenizer::from_str(schema);
        let mut res = Vec::new();
        for token in iter {
            res.push(token);
        }
        assert_eq!(
            res,
            vec![
                "message", "schema", "{", "required", "int32", "a", ";", "optional", "binary", "c",
                "(", "UTF8", ")", ";", "required", "group", "d", "{", "required", "int32", "a",
                ";", "optional", "binary", "c", "(", "UTF8", ")", ";", "}", "required", "group",
                "e", "(", "LIST", ")", "{", "repeated", "group", "list", "{", "required", "int32",
                "element", ";", "}", "}", "}"
            ]
        );
    }

    #[test]
    fn test_assert_token() {
        assert!(assert_token(Some("a"), "a").is_ok());
        assert!(assert_token(Some("a"), "b").is_err());
        assert!(assert_token(None, "b").is_err());
    }

    #[test]
    fn test_parse_message_type_invalid() {
        let mut iter = Tokenizer::from_str("test");
        let result = Parser {
            tokenizer: &mut iter,
        }
        .parse_message_type();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Parquet error: Message type does not start with 'message'"
        );
    }

    #[test]
    fn test_parse_message_type_no_name() {
        let mut iter = Tokenizer::from_str("message");
        let result = Parser {
            tokenizer: &mut iter,
        }
        .parse_message_type();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Parquet error: Expected name, found None"
        );
    }

    #[test]
    fn test_parse_message_type_fixed_byte_array() {
        let schema = "
    message schema {
      REQUIRED FIXED_LEN_BYTE_ARRAY col;
    }
    ";
        let mut iter = Tokenizer::from_str(schema);
        let result = Parser {
            tokenizer: &mut iter,
        }
        .parse_message_type();
        assert!(result.is_err());

        let schema = "
    message schema {
      REQUIRED FIXED_LEN_BYTE_ARRAY(16) col;
    }
    ";
        let mut iter = Tokenizer::from_str(schema);
        let result = Parser {
            tokenizer: &mut iter,
        }
        .parse_message_type();
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_message_type_decimal() {
        // It is okay for decimal to omit precision and scale with right syntax.
        // Here we test wrong syntax of decimal type

        // Invalid decimal syntax
        let schema = "
    message root {
      optional int32 f1 (DECIMAL();
    }
    ";
        let mut iter = Tokenizer::from_str(schema);
        let result = Parser {
            tokenizer: &mut iter,
        }
        .parse_message_type();
        assert!(result.is_err());

        // Invalid decimal, need precision and scale
        let schema = "
    message root {
      optional int32 f1 (DECIMAL());
    }
    ";
        let mut iter = Tokenizer::from_str(schema);
        let result = Parser {
            tokenizer: &mut iter,
        }
        .parse_message_type();
        assert!(result.is_err());

        // Invalid decimal because of `,` - has precision, needs scale
        let schema = "
    message root {
      optional int32 f1 (DECIMAL(8,));
    }
    ";
        let mut iter = Tokenizer::from_str(schema);
        let result = Parser {
            tokenizer: &mut iter,
        }
        .parse_message_type();
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_decimal_wrong() {
        // Invalid decimal because, we always require either precision or scale to be
        // specified as part of converted type
        let schema = "
    message root {
      optional int32 f3 (DECIMAL);
    }
    ";
        let mut iter = Tokenizer::from_str(schema);
        let result = Parser {
            tokenizer: &mut iter,
        }
        .parse_message_type();
        assert!(result.is_err());

        // Valid decimal (precision, scale)
        let schema = "
    message root {
      optional int32 f1 (DECIMAL(8, 3));
      optional int32 f2 (DECIMAL(8));
    }
    ";
        let mut iter = Tokenizer::from_str(schema);
        let result = Parser {
            tokenizer: &mut iter,
        }
        .parse_message_type();
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_message_type_compare_1() -> Result<()> {
        let schema = "
    message root {
      optional fixed_len_byte_array(5) f1 (DECIMAL(9, 3));
      optional fixed_len_byte_array (16) f2 (DECIMAL (38, 18));
    }
    ";
        let mut iter = Tokenizer::from_str(schema);
        let message = Parser {
            tokenizer: &mut iter,
        }
        .parse_message_type()
        .unwrap();

        let fields = vec![
            ParquetType::try_from_primitive(
                "f1".to_string(),
                PhysicalType::FixedLenByteArray(5),
                Repetition::Optional,
                Some(PrimitiveConvertedType::Decimal(9, 3)),
                None,
            )?,
            ParquetType::try_from_primitive(
                "f2".to_string(),
                PhysicalType::FixedLenByteArray(16),
                Repetition::Optional,
                Some(PrimitiveConvertedType::Decimal(38, 18)),
                None,
            )?,
        ];

        let expected = ParquetType::from_fields("root".to_string(), fields);

        assert_eq!(message, expected);
        Ok(())
    }

    #[test]
    fn test_parse_message_type_compare_2() -> Result<()> {
        let schema = "
    message root {
      required group a0 {
        optional group a1 (LIST) {
          repeated binary a2 (UTF8);
        }

        optional group b1 (LIST) {
          repeated group b2 {
            optional int32 b3;
            optional double b4;
          }
        }
      }
    }
    ";
        let mut iter = Tokenizer::from_str(schema);
        let message = Parser {
            tokenizer: &mut iter,
        }
        .parse_message_type()
        .unwrap();

        let a2 = ParquetType::try_from_primitive(
            "a2".to_string(),
            PhysicalType::ByteArray,
            Repetition::Repeated,
            Some(PrimitiveConvertedType::Utf8),
            None,
        )?;
        let a1 = ParquetType::from_converted(
            "a1".to_string(),
            vec![a2],
            None,
            Some(GroupConvertedType::List),
            None,
        );
        let b2 = ParquetType::from_converted(
            "b2".to_string(),
            vec![
                ParquetType::from_physical("b3".to_string(), PhysicalType::Int32),
                ParquetType::from_physical("b4".to_string(), PhysicalType::Double),
            ],
            Some(Repetition::Repeated),
            None,
            None,
        );
        let b1 = ParquetType::from_converted(
            "b1".to_string(),
            vec![b2],
            None,
            Some(GroupConvertedType::List),
            None,
        );
        let a0 = ParquetType::from_converted(
            "a0".to_string(),
            vec![a1, b1],
            Some(Repetition::Required),
            None,
            None,
        );

        let expected = ParquetType::from_fields("root".to_string(), vec![a0]);

        assert_eq!(message, expected);
        Ok(())
    }

    /*
    #[test]
    fn test_parse_message_type_compare_3() {
        let schema = "
    message root {
      required int32 _1 (INT_8);
      required int32 _2 (INT_16);
      required float _3;
      required double _4;
      optional int32 _5 (DATE);
      optional binary _6 (UTF8);
    }
    ";
        let mut iter = Tokenizer::from_str(schema);
        let message = Parser {
            tokenizer: &mut iter,
        }
        .parse_message_type()
        .unwrap();

        let mut fields = vec![
            Arc::new(
                ParquetType::primitive_type_builder("_1", Type::INT32)
                    .with_repetition(Repetition::REQUIRED)
                    .with_converted_type(ConvertedType::INT_8)
                    .build()
                    .unwrap(),
            ),
            Arc::new(
                ParquetType::primitive_type_builder("_2", Type::INT32)
                    .with_repetition(Repetition::REQUIRED)
                    .with_converted_type(ConvertedType::INT_16)
                    .build()
                    .unwrap(),
            ),
            Arc::new(
                ParquetType::primitive_type_builder("_3", Type::FLOAT)
                    .with_repetition(Repetition::REQUIRED)
                    .build()
                    .unwrap(),
            ),
            Arc::new(
                ParquetType::primitive_type_builder("_4", Type::DOUBLE)
                    .with_repetition(Repetition::REQUIRED)
                    .build()
                    .unwrap(),
            ),
            Arc::new(
                ParquetType::primitive_type_builder("_5", Type::INT32)
                    .with_converted_type(ConvertedType::DATE)
                    .build()
                    .unwrap(),
            ),
            Arc::new(
                ParquetType::primitive_type_builder("_6", Type::BYTE_ARRAY)
                    .with_converted_type(ConvertedType::UTF8)
                    .build()
                    .unwrap(),
            ),
        ];

        let expected = ParquetType::from_fields("root", fields);
        assert_eq!(message, expected);
    }
     */
}