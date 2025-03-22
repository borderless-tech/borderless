#![allow(dead_code, unused)]

pub struct Response<T> {
    pub status: u16,
    pub result: Option<T>,
}

// Suuuuuper basic, just to get started
pub trait HttpService {
    type Output;

    fn get(path: &str, query: Option<&str>) -> Response<Self::Output>;

    fn post(path: &str, query: Option<&str>, payload: Vec<u8>) -> Response<Self::Output>;
}

pub trait Field {
    fn get(&self, path: &str, query: Option<&str>) -> Response<String>;
}

impl Field for bool {
    fn get(&self, path: &str, _query: Option<&str>) -> Response<String> {
        match path {
            "" | "/" => Response {
                status: 200,
                result: Some(format!("{self}")),
            },
            _ => Response {
                status: 404,
                result: None,
            },
        }
    }
}

impl Field for u32 {
    fn get(&self, path: &str, _query: Option<&str>) -> Response<String> {
        match path {
            "" | "/" => Response {
                status: 200,
                result: Some(format!("{self}")),
            },
            _ => Response {
                status: 404,
                result: None,
            },
        }
    }
}

// Okay, Crazy Idea:
//
// If we want to generate our rest-api, we basically want to convert path-segments in the url to field-names
// of nested structs. Consider the following ( just to have some example types ):
//
// struct Foo { number: u32, msg: String }
// struct Baa { foo: Foo, info: String }
//
// struct State { baa: Baa, foo: Foo, nothing: u32 }
//
// The State should produce the following routes:
//
// /baa            -> state.baa
// /baa/foo        -> state.baa.foo
// /baa/foo/number -> state.baa.foo.number
// /baa/foo/msg    -> state.baa.foo.msg
// /baa/info       -> state.baa.info
// /foo            -> state.foo
// /foo/number     -> state.foo.number
// /foo/msg        -> state.foo.msg
// /nothing        -> state.nothing
//
// What if we could utilize the serde-data-model here, to match and check, which field must be loaded ?
//
// Example: We have a 'get' request on this path '/baa/foo/msg'
// 1. Call our serializer method for State with path /baa/foo/msg, start with first path-segment
// 2. /baa -> look for datatype of field baa and call method for the Type of baa ( Baa )
// 3. /foo -> look for datatype of field foo and call method for the Type of foo ( Foo )
// 4. /msg -> look for datatype of field msg and call method for the Type of msg ( String )
// 5. "" -> Serialize self (msg: String)
//
// This is similar to how serde internally works (because we call the .serialize method on every type and field),
// the only difference now is, that we force our serializer to ignore fields, that are out of its scope;
// meaning whose field names do not match the path '/baa/foo/msg'
//
// We could directly incorporate the outgoing format, that we want (e.g. directly serialize to json);
// or we could parse to an intermediate type here, like serde_json::Value.
//
// NOTE: In the end, I think we should wrap this into our own trait, to make things like the binary-tree
// implement this, although they don't implement the serde traits (because they are not serializable).
// We could automatically implement our trait for every type that implements the serde traits,
// to avoid having another #derive macro on top of every datatype
//
//
// -> As far as I understand this, we could even go completely crazy here, and have an inner serializer
// inside our serializer, that we use to actually serialize the value,
// while we use the outer serializer only to keep track of the nested state.
// (But I am not completely sure that this is possible)

use std::fmt::Display;

use serde::{ser, Serialize};
use serde_json::Serializer;

pub struct PathSerializer<W> {
    // json serializer
    json: Serializer<W>,
    ctx: PathCtx,
}

pub struct PathCtx {
    output: String,
    // Input path that the serializer should use to match against
    segments: Vec<String>,
    // Currently indexed element
    seg_idx: usize,
    // Indicate, that we are done, because we have reached the end of our serializer
    done: bool,
    full_match: bool,
}

pub struct PathStructSerializer<'a, W: std::io::Write> {
    // json: <&'a mut Serializer<W> as ser::Serializer>::SerializeTupleStruct,
    json: &'a mut Serializer<W>,
    ctx: &'a mut PathCtx,
    name: &'static str,
    len: usize,
}

impl<W> PathSerializer<W>
where
    W: std::io::Write,
{
    pub fn from_path(path: &str, writer: W) -> Self {
        let segments = path
            .split('/')
            .flat_map(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            })
            .collect();
        println!("starting to serialize for path: {path}");
        PathSerializer {
            json: Serializer::new(writer),
            ctx: PathCtx {
                output: String::new(),
                segments,
                seg_idx: 0,
                done: false,
                full_match: false,
            },
        }
    }
}
pub type Result<T> = std::result::Result<T, serde_json::Error>;

// By convention, the public API of a Serde serializer is one or more `to_abc`
// functions such as `to_string`, `to_bytes`, or `to_writer` depending on what
// Rust types the serializer is able to produce as output.
//
// This basic serializer supports only `to_string`.
pub fn to_string<T>(value: &T, path: &str) -> Result<Option<String>>
where
    T: Serialize,
{
    // Different Approach:
    let value = serde_json::to_value(value)?;

    let mut current = &value;
    for seg in path
        .split('/')
        .flat_map(|s| if s.is_empty() { None } else { Some(s) })
    {
        current = match current.get(seg) {
            Some(v) => v,
            None => return Ok(None),
        };
    }
    // FCK my life, this works so great...
    Ok(Some(current.to_string()))
}
