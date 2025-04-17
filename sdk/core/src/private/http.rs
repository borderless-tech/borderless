use serde::Serialize;

// NOTE: We need something different, that works with our storage implementation.
// For now, we can try fiddling around with this.
pub fn to_payload<T>(value: &T, path: &str) -> anyhow::Result<Option<String>>
where
    T: Serialize,
{
    // Different Approach:
    let value = serde_json::to_value(value)?;

    // Instantly return the value
    if path.is_empty() {
        return Ok(Some(value.to_string()));
    }

    // Search sub-fields based on path
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

// /// Extracts a sub-section from the given value based on the given url path
// pub fn to_payload<T>(value: &T, path: &str) -> anyhow::Result<Option<serde_json::Value>>
// where
//     T: Serialize,
// {
//     // Different Approach:
//     let value = serde_json::to_value(value)?;

//     let mut current = &value;
//     for seg in path
//         .split('/')
//         .flat_map(|s| if s.is_empty() { None } else { Some(s) })
//     {
//         current = match current.get(seg) {
//             Some(v) => v,
//             None => return Ok(None),
//         };
//     }
//     // FCK my life, this works so great...
//     Ok(Some(current.clone()))
// }
