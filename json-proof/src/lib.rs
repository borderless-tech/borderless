use std::{cmp::Ordering, collections::VecDeque};

use borderless_hash::{Hash256, Hasher};
use error::ErrorImpl;
use serde_json::{Map, Value};

/// Arbitrary prefix that we use to distinguish "real" values in the document from the ones that we added.
///
/// In general, this value can be set to whatever, but it should most likely be no real json key.
pub const HASH_PREFIX: &str = "__sha3_hash_x";

pub mod error {
    use thiserror::Error;

    #[derive(Debug, Error)]
    #[error(transparent)]
    pub struct Error {
        inner: ErrorImpl,
    }

    #[derive(Debug, Error)]
    pub(crate) enum ErrorImpl {
        #[error("(de-)serialization error - {0}")]
        Serde(#[from] serde_json::Error),
        #[error("expected JSON object")]
        NotAnObject,
        #[error("expected string")]
        NotAString,
        #[error("missing key '{0}'")]
        MissingKey(String),
        #[error("invalid type for key '{0}' in mask: Expected boolean")]
        InvalidMaskKey(String),
        #[error("invalid hash - {0}")]
        InvalidHash(#[from] base16::DecodeError),
        #[error(
            "cannot build proof from object that contains both obfuscated and unobfuscated key"
        )]
        SameKey,
    }

    impl From<ErrorImpl> for Error {
        fn from(value: ErrorImpl) -> Self {
            Self { inner: value }
        }
    }

    impl From<serde_json::Error> for Error {
        fn from(value: serde_json::Error) -> Self {
            let inner = value.into();
            Self { inner }
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
pub use error::Error;

/// Generates the json-proof for some serializable value
///
/// Note: If the value is not an object, this function will fail.
pub fn generate_proof_for_obj<V>(value: &V) -> Result<Hash256>
where
    V: serde::Serialize,
{
    let value = serde_json::to_value(value)?;
    let mut canonicalized = canonicalize_json(value)?;
    let encoded_proof = gen_proof(&mut canonicalized)?;
    let mut out = [0; 32];
    base16::decode_slice(&encoded_proof, &mut out).map_err(ErrorImpl::InvalidHash)?;
    Ok(out.into())
}

/// Processes the json value and re-encodes it in a canonical way.
///
/// This process ensures, that the output document will always be the same,
/// even if the ordering of object keys is changed in the input object
/// (as those things are still the same json document).
///
/// In general, this function applies an ordering to all object keys.
/// Things like lists define their own ordering and are not changed by this function.
pub fn canonicalize_json(value: Value) -> Result<Map<String, Value>> {
    let mut value = json_syntax::Value::from_serde_json(value);
    value.canonicalize();
    let map = match value.into_serde_json() {
        Value::Object(map) => map,
        _ => return Err(ErrorImpl::NotAnObject.into()),
    };
    // map.values_mut().for_each(Value::sort_all_objects);
    Ok(map)
}

/// Calculates the hash of some json value
///
/// Also includes the key of the value in the hash calculation,
/// to avoid different json objects with the same content evaluating to the same hash.
fn hash_value(key: &str, value: &Value) -> Hash256 {
    let mut hasher = Hasher::new();

    match value {
        // For objects, the hash is the hash of all keys
        // NOTE: For reproducible results, you have to sort the keys first
        Value::Object(map) => {
            for (key, value) in map.iter() {
                if key.starts_with(HASH_PREFIX) {
                    continue;
                }
                let hash = hash_value(key, value);
                hasher.update(&hash);
            }
        }
        // For everything else, we just convert the value to a string, and hash this
        other => {
            // This may be redundant, but just to be extra sure
            let mut canonical = json_syntax::Value::from_serde_json(other.clone());
            canonical.canonicalize();
            let string = canonical.to_string();
            hasher.update(&key.as_bytes());
            hasher.update(&string.as_bytes());
        }
    }
    let digest = hasher.finalize();
    digest.into()
}

/// Calculates the proofs for every member of the JSON object.
///
/// This also includes the object itself. The hash of the root object is stored at the key `{HASH_PREFIX}___self___`.
///
/// Note: This function mutates the json object and adds the proof keys directly to it.
pub fn prepare_document(map: &mut Map<String, Value>) {
    let mut append_keys = Vec::new();
    let self_hash = hash_value("self", &Value::Object(map.clone()));
    let encoded_self = base16::encode_lower(&self_hash);
    append_keys.push((format!("{HASH_PREFIX}___self___"), encoded_self));

    for (key, value) in map.iter_mut() {
        let hash = hash_value(key, value);
        let encoded = base16::encode_lower(&hash);
        append_keys.push((format!("{HASH_PREFIX}_{key}"), encoded));
        // Nest one level deeper
        if let Value::Object(nested) = value {
            prepare_document(nested);
        }
    }
    // Add the hashes afterwards, to not mess with the object while we are still recursing
    for (key, hash) in append_keys {
        map.insert(key, Value::String(hash));
    }
}

/// Removes all metadata-keys, that we added ourself during the preparation.
pub fn split_out_proof(map: &mut Map<String, Value>) -> Map<String, Value> {
    let mut out = Map::new();
    let mut remove_keys = Vec::new();
    for (key, value) in map.iter_mut() {
        if key.starts_with(HASH_PREFIX) {
            remove_keys.push(key.clone());
            out.insert(key.clone(), value.clone());
        }
        // Nest one level deeper
        if let Value::Object(nested) = value {
            let obj = split_out_proof(nested);
            out.insert(key.clone(), Value::Object(obj));
        }
    }
    for key in remove_keys {
        map.remove(&key);
    }
    out
}

/// Generates a mask for the given object, that can be used to add or remove values from the final document.
///
/// Basically replaces every value (that is not an object) with a boolean
pub fn generate_mask(map: &Map<String, Value>) -> Map<String, Value> {
    let mut out = Map::new();
    for (key, value) in map.iter() {
        if let Value::Object(obj) = value {
            let mask = generate_mask(obj);
            out.insert(key.clone(), Value::Object(mask));
        } else {
            out.insert(key.clone(), Value::Bool(true));
        }
    }
    out
}

/// Checks, weather or not a mask belongs to a given object
pub fn check_mask(mask: &Map<String, Value>, obj: &Map<String, Value>) -> Result<()> {
    for (key, value) in obj.iter() {
        let mask_value = match mask.get(key) {
            Some(v) => v,
            None => return Err(ErrorImpl::MissingKey(key.to_string()).into()),
        };
        match (mask_value, value) {
            (Value::Object(mask_obj), Value::Object(nested)) => {
                // The mask contains a full object that specifies which keys should be present
                check_mask(mask_obj, nested)?;
            }
            (Value::Bool(_), _) => continue, // valid case
            (_invalid_type, _) => return Err(ErrorImpl::InvalidMaskKey(key.to_string()).into()),
        }
    }
    Ok(())
}

/// Applies a mask to some given object using given proof
///
/// # Safety
///
/// The mask must belong to the object and the proof must be generated from the object,
/// otherwise this function will panic.
/// Since the function calls itself recursively, the check is outsourced to a separate function.
/// Always use [`check_mask`] before calling.
pub fn apply_mask(
    mask: &Map<String, Value>,
    obj: &Map<String, Value>,
    proof: &Map<String, Value>,
) -> Map<String, Value> {
    let mut out = Map::new();

    for (key, value) in mask.iter() {
        let obj_value = obj.get(key).expect("missing key in object");
        let proof_key = format!("{HASH_PREFIX}_{key}");
        let proof_value = proof.get(&proof_key).expect("missing key in proof");
        match value {
            Value::Bool(insert) => {
                if *insert {
                    out.insert(key.clone(), obj_value.clone());
                } else {
                    out.insert(proof_key, proof_value.clone());
                }
            }
            Value::Object(nested_mask) => {
                let proof_obj = proof.get(key).expect("missing key in proof");
                match (obj_value, proof_obj) {
                    (Value::Object(nested_obj), Value::Object(nested_proof)) => {
                        let nested = apply_mask(nested_mask, nested_obj, nested_proof);
                        out.insert(key.clone(), Value::Object(nested));
                    }
                    (nested_obj, nested_proof) => {
                        println!("object: {}", nested_obj);
                        println!("proof: {}", nested_proof);
                        panic!("datatype mismatch - expected object");
                    }
                }
            }
            _ => panic!("invalid type in mask"),
        }
    }
    out
}

/// Helper function that checks, if any key in the object starts with `HASH_PREFIX`
fn contains_prefix(map: &Map<String, Value>) -> bool {
    for (key, value) in map.iter() {
        if key.starts_with(HASH_PREFIX) {
            return true;
        }
        if let Value::Object(obj) = value {
            if contains_prefix(obj) {
                return true;
            }
        }
    }
    false
}

/// Decodes the hash from an obfuscated value.
fn hash_from_obfuscated(value: &Value) -> Result<Hash256> {
    if let Value::String(s) = value {
        let mut out = [0; 32];
        base16::decode_slice(s, &mut out).map_err(ErrorImpl::InvalidHash)?;
        Ok(out.into())
    } else {
        Err(ErrorImpl::NotAString.into())
    }
}

/// Rebuilds the proof for some JSON object.
///
/// This is meant to be called with an object, that contains prefixed (obfuscated) values.
/// If this function is called with a normal JSON object, it will simply calculate the proof for this object.
/// In those cases, it might be better to just call [`prepare_document`] or [`hash_value`] directly.
///
/// # Errors
///
/// This function will fail, if the value under a `HASH_PREFIX` key is not a base-58 encoded sha3-256 hash.
fn rebuild_proof(map: &Map<String, Value>) -> Result<Hash256> {
    let mut unprefixed: VecDeque<_> = map
        .keys()
        .filter_map(|k| k.strip_prefix(&format!("{HASH_PREFIX}_")))
        .collect();
    // ensure that this is sorted, so our comparison below works as expected
    unprefixed.make_contiguous().sort();
    let mut next_obfuscated = unprefixed.pop_front();

    let mut hasher = Hasher::new();

    for (key, value) in map.iter() {
        // Ignore all prefixed keys
        if key.starts_with(HASH_PREFIX) {
            continue;
        }
        // Check, if the next unprefixed key would come first in our hash calculation
        while let Some(obfs) = next_obfuscated {
            match obfs.cmp(key) {
                Ordering::Less => {
                    // Use the hash from the prefixed key
                    let decoded = map.get(&format!("{HASH_PREFIX}_{obfs}")).unwrap();
                    let hash = hash_from_obfuscated(decoded)?;
                    hasher.update(&hash);
                }
                Ordering::Equal => {
                    return Err(ErrorImpl::SameKey.into());
                }
                Ordering::Greater => break,
            }
            next_obfuscated = unprefixed.pop_front();
        }
        // Start to calculate the hash of the current key
        //
        // Nest one level deeper, if required
        if let Value::Object(nested) = value {
            let hash = rebuild_proof(nested)?;
            hasher.update(&hash);
        } else {
            let hash = hash_value(key, value);
            hasher.update(&hash);
        }
    }
    // If there are still some prefixed-keys left, we also have to apply them
    while let Some(obfs) = next_obfuscated {
        // Use the hash from the prefixed key
        let decoded = map.get(&format!("{HASH_PREFIX}_{obfs}")).unwrap();
        let hash = hash_from_obfuscated(decoded)?;
        hasher.update(&hash);
        next_obfuscated = unprefixed.pop_front();
    }

    let digest = hasher.finalize();
    Ok(digest)
}

/// Generates a proof for some JSON object
///
/// It will return the base-58 encoded hash of the proof as a string.
///
/// # Errors
///
/// This function will fail, if the object contains obfuscated keys, whose values cannot be decoded into valid hashes.
/// This is never the case, if the object was created by one of our own functions, but only if someone has messed with the json.
pub fn gen_proof(map: &mut Map<String, Value>) -> Result<String> {
    // If this is a simple object, without any of our custom fields,
    // the proof is just the root-hash of the document
    if contains_prefix(map) {
        let proof = rebuild_proof(map)?;
        let encoded = base16::encode_lower(&proof);
        return Ok(encoded);
    }
    // Let's be extra paranoid here - in test mode, we double check,
    // that the generated proof and the "rebuild" proof are always identical.
    #[cfg(test)]
    {
        let proof = rebuild_proof(map)?;
        let encoded = base16::encode_lower(&proof);
        prepare_document(map);
        if let Value::String(s) = map.get(&format!("{HASH_PREFIX}___self___")).unwrap() {
            assert_eq!(encoded, *s);
            Ok(s.clone())
        } else {
            unreachable!("___self___ is always a string");
        }
    }
    #[cfg(not(test))]
    {
        prepare_document(map);
        if let Value::String(s) = map.get(&format!("{HASH_PREFIX}___self___")).unwrap() {
            Ok(s.clone())
        } else {
            unreachable!("___self___ is always a string");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use rand::{seq::SliceRandom, thread_rng, Rng};
    use serde_json::Number;

    const FLOAT_PRECISION: &str = r##"
{
  "a": 0.0,
  "b": 1.0,
  "c": -0.0,
  "d": 1e30,
  "e": 0.0000000000000001,
  "f": 3.141592653589793238462643383279,
  "g": 12345678901234567890
}"##;

    const UNICODE: &str = r##"
{
  "ascii": "hello",
  "escaped": "\u0061\u0062\u0063",
  "emoji": "😀",
  "combining": "e\u0301",
  "accented": "é"
}
"##;

    const UNORDERED: &str = r##"
{
  "z": 1,
  "a": 2,
  "c": 3,
  "b": 4
}
"##;

    const DUPLICATES: &str = r##"
{
  "x": 1,
  "x": 2
}
"##;

    const DEEP_NEST: &str = r##"
{
  "a": {
    "b": {
      "c": {
        "d": {
          "e": {
            "f": [1, 2, {"g": "end"}]
          }
        }
      }
    }
  }
}
"##;

    const TYPES: &str = r##"
{
  "bool": true,
  "null": null,
  "number": 42,
  "string": "42",
  "array": [1, "2", false],
  "object": {
    "nested": "yes"
  }
}"##;

    const STRINGS: &str = r##"
{
  "int_string": "00042",
  "float_string": "1.000",
  "hex_string": "0xDEAD",
  "zero_string": "0",
  "true_string": "true"
}"##;

    const EMPTY: &str = r##"
{
  "empty_array": [],
  "empty_object": {},
  "zero": 0,
  "false": false,
  "empty_string": ""
}"##;

    const ARRAY_ORDER: &str = r##"
{
  "array": [3, 1, 2, 4, 5]
}
"##;

    /// Returns a new `Map` with keys shuffled (useful for canonicalization testing)
    fn shuffle_json_object(input: &Map<String, Value>) -> Map<String, Value> {
        let mut entries: Vec<(String, Value)> =
            input.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        entries.shuffle(&mut thread_rng());

        let mut shuffled = Map::new();
        for (k, v) in entries {
            let new_value = match v {
                // If the value is a nested object, recursively shuffle it
                Value::Object(ref obj) => Value::Object(shuffle_json_object(obj)),
                // If it's an array of objects, optionally shuffle inside each
                Value::Array(arr) => Value::Array(
                    arr.into_iter()
                        .map(|elem| match elem {
                            Value::Object(ref inner_obj) => {
                                Value::Object(shuffle_json_object(inner_obj))
                            }
                            other => other,
                        })
                        .collect(),
                ),
                other => other,
            };
            shuffled.insert(k, new_value);
        }
        shuffled
    }

    /// Generates a random JSON object with some depth and variety
    fn generate_random_json(depth: usize) -> Value {
        let mut rng = thread_rng();
        if depth == 0 {
            // Base case: return a primitive
            let integer: i64 = rng.gen_range(0..100).into();
            let float: f64 = rng.gen_range(-1.5..1e9);
            let primitives = vec![
                Value::Null,
                Value::Bool(rng.gen_bool(0.5)),
                Value::Number(integer.into()),
                Value::Number(Number::from_f64(float).unwrap()),
                Value::String((0..5).map(|_| rng.gen_range('a'..='z')).collect()),
            ];
            return primitives.choose(&mut rng).unwrap().clone();
        }

        let mut obj = Map::new();
        let num_keys = rng.gen_range(2..6);

        for _ in 0..num_keys {
            let key = (0..rng.gen_range(1..5))
                .map(|_| rng.gen_range('a'..='z'))
                .collect::<String>();
            let value_type = rng.gen_range(0..3);

            let value = match value_type {
                0 => generate_random_json(depth - 1), // Nested object or primitive
                1 => Value::Array(
                    (0..rng.gen_range(1..4))
                        .map(|_| generate_random_json(depth - 1))
                        .collect(),
                ),
                _ => Value::Number(rng.gen_range(0..10000).into()),
            };

            obj.insert(key, value);
        }

        Value::Object(obj)
    }

    fn generate_float_json() -> Value {
        let mut rng = thread_rng();

        let mut obj = Map::new();
        let num_keys = rng.gen_range(20..80);

        let gen_float = || {
            let mut rng = thread_rng();
            let float_32 = Number::from_f64(rng.gen::<f32>().into()).unwrap();
            let float_64 = Number::from_f64(rng.gen()).unwrap();
            let options = [Value::Number(float_32), Value::Number(float_64)];
            return options.choose(&mut rng).unwrap().clone();
        };

        for _ in 0..num_keys {
            let key = (0..rng.gen_range(1..5))
                .map(|_| rng.gen_range('a'..='z'))
                .collect::<String>();
            let value = gen_float();
            obj.insert(key, value);
        }
        obj.into()
    }

    /// Executes a test-case against a test-string and tests against given proof-value.
    fn test_case(test_string: &str, proof_value: &str, test_repeats: usize) -> Result<()> {
        for _ in 0..test_repeats {
            // Base value
            let value = serde_json::from_str(test_string)?;
            let mut canonicalized = canonicalize_json(value)?;
            // Shuffled value
            let mut shuffled = shuffle_json_object(&canonicalized);
            let mut re_canonicalized = canonicalize_json(shuffled.clone().into())?;
            // Re-load after serializing shuffled value
            let shuffled_value: Value = shuffled.clone().into();
            let reloaded = Value::from_str(&shuffled_value.to_string())?;
            let mut reloaded = canonicalize_json(reloaded)?;

            let proof = gen_proof(&mut canonicalized)?;
            let proof_shuffled = gen_proof(&mut shuffled)?;
            let proof_re_canon = gen_proof(&mut re_canonicalized)?;
            let proof_reloaded = gen_proof(&mut reloaded)?;

            assert_eq!(proof, proof_value);
            assert_eq!(proof_re_canon, proof);
            assert_eq!(proof_re_canon, proof_value);
            assert_eq!(proof_reloaded, proof_value);
            // If the shuffled version equals the un-shuffled, then the proofs must match
            if shuffled != re_canonicalized {
                assert_ne!(proof_shuffled, proof);
            } else {
                assert_eq!(proof_shuffled, proof);
            }
        }
        Ok(())
    }

    #[test]
    fn float_precision() -> Result<()> {
        test_case(
            FLOAT_PRECISION,
            "058c5db7818861ccf63cb8c3a5de85c756cdc32c9f0bbb8677b446373992f672",
            100,
        )
    }

    #[test]
    fn unicode() -> Result<()> {
        test_case(
            UNICODE,
            "99dac1e3f2deea4b55358318679f2ac33b211e360191e7f04ea23cc9dbffa5e5",
            100,
        )
    }

    #[test]
    fn duplicate_keys() -> Result<()> {
        test_case(
            DUPLICATES,
            "38bdd7729704ef6f7e5467eacde43d62c02bd42d126cad3942ebbaae17d57555",
            100,
        )
    }

    #[test]
    fn unordered_keys() -> Result<()> {
        test_case(
            UNORDERED,
            "8b4abac49295aee61093583e01be00c8e11b06585e617ce3a99b2587d7faf3c5",
            100,
        )
    }

    #[test]
    fn deep_nesting() -> Result<()> {
        test_case(
            DEEP_NEST,
            "4785149ec8ed5e9f6e97e7014522493105e0016032b56e84314f62838552745d",
            100,
        )
    }

    #[test]
    fn type_fidelity() -> Result<()> {
        test_case(
            TYPES,
            "2672def9f1fd6657c653a1d6e60ee497e8a426f4e2b85552f9e97c35b973c83f",
            100,
        )
    }

    #[test]
    fn special_strings() -> Result<()> {
        test_case(
            STRINGS,
            "c6222174262b9c37f282530d6525dd6333f99f22a8a65cb6749445f07105b670",
            100,
        )
    }

    #[test]
    fn empty_values() -> Result<()> {
        test_case(
            EMPTY,
            "6b646fa0961c777be9d93ddc7c3bf52ee83bc56de36de5824b0656895375c4d5",
            100,
        )
    }

    #[test]
    fn array_ordering() -> Result<()> {
        test_case(
            ARRAY_ORDER,
            "01aa1e0cdb3c047651fbc565816d4bbf4fb51a4f022b13ac157aefd6f98e4bb7",
            100,
        )
    }

    const EQUIV_1: &str = r##"{ "x": 1, "y": 2 }"##;
    const EQUIV_2: &str = r##"{ "y": 2, "x": 1 }"##;

    #[test]
    fn equivalent_documents() -> Result<()> {
        let v1 = serde_json::from_str(EQUIV_1)?;
        let v2 = serde_json::from_str(EQUIV_2)?;
        let mut c1 = canonicalize_json(v1)?;
        let mut c2 = canonicalize_json(v2)?;
        let p1 = gen_proof(&mut c1)?;
        let p2 = gen_proof(&mut c2)?;
        assert_eq!(p1, p2);
        Ok(())
    }

    #[test]
    fn non_object_input_fails() {
        let invalid = r#""just a string""#;
        let result = canonicalize_json(serde_json::from_str(invalid).unwrap());
        assert!(result.is_err(), "Expected error for non-object root");
    }

    #[test]
    fn fuzz_random_json_consistency() -> Result<()> {
        for _ in 0..100 {
            let original = generate_random_json(6);
            let mut canonicalized = canonicalize_json(original.clone())?;
            let proof = gen_proof(&mut canonicalized)?;

            // Shuffle the object
            if let Value::Object(obj) = original {
                let shuffled = shuffle_json_object(&obj);
                let mut re_canon = canonicalize_json(Value::Object(shuffled))?;
                let re_proof = gen_proof(&mut re_canon)?;

                assert_eq!(proof, re_proof, "Hash mismatch on fuzzed input");
            }
        }
        Ok(())
    }

    #[test]
    fn fuzz_random_json_re_parsed() -> Result<()> {
        for _ in 0..10 {
            let original = generate_random_json(8);
            let encoded = original.to_string();

            let reloaded = serde_json::Value::from_str(&encoded)?;

            let mut canonicalized_1 = canonicalize_json(original)?;
            let mut canonicalized_2 = canonicalize_json(reloaded)?;
            let proof_1 = gen_proof(&mut canonicalized_1)?;
            let proof_2 = gen_proof(&mut canonicalized_2)?;
            assert_eq!(proof_1, proof_2);
        }
        Ok(())
    }

    #[test]
    fn fuzz_base_test_l1() -> Result<()> {
        for _ in 0..100 {
            let original = generate_random_json(1);
            let mut canonicalized = canonicalize_json(original.clone())?;
            let proof = gen_proof(&mut canonicalized)?;
            test_case(&original.to_string(), &proof, 10)?;
        }
        Ok(())
    }

    #[test]
    fn fuzz_base_test_l2() -> Result<()> {
        for _ in 0..50 {
            let original = generate_random_json(2);
            let mut canonicalized = canonicalize_json(original.clone())?;
            let proof = gen_proof(&mut canonicalized)?;
            test_case(&original.to_string(), &proof, 10)?;
        }
        Ok(())
    }

    #[test]
    fn fuzz_base_test_l3() -> Result<()> {
        for _ in 0..25 {
            let original = generate_random_json(3);
            let mut canonicalized = canonicalize_json(original.clone())?;
            let proof = gen_proof(&mut canonicalized)?;
            test_case(&original.to_string(), &proof, 10)?;
        }
        Ok(())
    }

    #[test]
    fn fuzz_float_values() -> Result<()> {
        for _ in 0..100 {
            let original = generate_float_json();
            let mut canonicalized = canonicalize_json(original.clone())?;
            let proof = gen_proof(&mut canonicalized)?;
            test_case(&original.to_string(), &proof, 10)?;
        }
        Ok(())
    }
}
