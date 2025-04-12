use serde_json::Value;

/// Finds a JSON element at a specific position within a nested JSON structure.
///
/// This function traverses a JSON structure (containing objects and arrays) in a depth-first manner,
/// counting elements as it goes, until it reaches the element at the specified position.
///
/// # Arguments
///
/// * `steps` - The position (zero-based) to locate in the JSON structure.
/// * `obj` - The JSON value to traverse, typically the root of the JSON structure.
///
/// # Returns
///
/// A tuple containing:
/// * `Option<&Value>` - The parent object/array containing the element at the specified position,
///    or `None` if the position is out of bounds.
/// * `usize` - The index of the element within its parent.
///
/// # Examples
///
/// ```
/// use serde_json::json;
///
/// let mut value = json!({
///     "name": "Jane Doe",         // position 0
///     "address": {                // position 1
///         "street": "123 Main St", // position 2
///         "city": "Anytown"        // position 3
///     },
///     "hobbies": [                // position 4
///         "reading",              // position 5
///         "hiking"                // position 6
///     ]
/// });
///
/// // Find the "address" object
/// let (obj_ref, index) = get_nested_object_to_insert_into(1, &mut value);
/// assert!(obj_ref.is_some());
/// assert_eq!(index, 1); // "address" is the second key in the root object
///
/// // Find the "hobbies" array
/// let (obj_ref, index) = get_nested_object_to_insert_into(4, &mut value);
/// assert!(obj_ref.is_some());
/// assert_eq!(index, 2); // "hobbies" is the third key in the root object
///
/// // Find an element in the array
/// let (obj_ref, index) = get_nested_object_to_insert_into(6, &mut value);
/// assert!(obj_ref.is_some());
/// let array = obj_ref.unwrap().as_array().unwrap();
/// assert_eq!(array[index].as_str().unwrap(), "hiking");
///
/// // Position beyond the structure returns None
/// let (obj_ref, _) = get_nested_object_to_insert_into(10, &mut value);
/// assert!(obj_ref.is_none());
/// ```
///
/// # Note
///
/// - Positions are counted in a depth-first traversal order.
/// - The function returns the parent container, not the element itself.
/// - For performance reasons, no validation is performed on the input JSON structure.
pub fn get_nested_object_to_insert_into<'og>(steps: usize, obj: &'og mut Value) -> (Option<&'og mut Value>, usize) {

    fn find_path(current: &mut usize, target: usize, value: &Value) -> Option<Vec<usize>> {
        match value {
            Value::Object(map) => {
                for (i, (_, v)) in map.iter().enumerate() {
                    if *current == target {
                        return Some(vec![i]);
                    }

                    *current += 1;
                    
                    if v.is_object() || v.is_array() {
                        if let Some(mut path) = find_path(current, target, v) {
                            path.insert(0, i);
                            return Some(path);
                        }
                    }
                }
                None
            },
            Value::Array(arr) => {
                for (i, v) in arr.iter().enumerate() {
                    if *current == target && !v.is_object() && !v.is_array() {
                        return Some(vec![i]);
                    }
                    
                    if v.is_object() || v.is_array() {
                        if let Some(mut path) = find_path(current, target, v) {
                            path.insert(0, i);
                            return Some(path);
                        }
                    } else {
                        *current += 1;
                    }
                }
                None
            },
            _ => None
        }
    }
    
    // Now follow the path to get a mutable reference
    fn follow_path<'a>(obj: &'a mut Value, path: &[usize]) -> (Option<&'a mut Value>, usize) {
        if path.is_empty() {
            return (None, 0);
        }
        
        let idx = path[0];
        
        if path.len() == 1 {
            return (Some(obj), idx);
        }
        
        match obj {
            Value::Object(map) => {
                let key = map.keys().nth(idx).cloned();
                if let Some(key) = key {
                    if let Some(value) = map.get_mut(&key) {
                        return follow_path(value, &path[1..]);
                    }
                }
            },
            Value::Array(arr) => {
                if idx < arr.len() {
                    if let Some(value) = arr.get_mut(idx) {
                        return follow_path(value, &path[1..]);
                    }
                }
            },
            _ => {}
        }
        
        (None, 0)
    }
    
    let mut current = 0;
    let path = find_path(&mut current, steps, obj);
    
    return if let Some(path) = path {
        follow_path(obj, &path)
    } else {
        (None, 0)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_get_nested_object_to_insert_into_basic() {
        let mut value = json!({
            "name": "Jane Doe", // 0
            "age": 9, // 1
            "address": { // 2
                "street": "123 Main St", // 3
                "city": "Anytown", // 4
                "state": "CA", // 5
                "zip": "12345" // 6
            },
            "salary": "5", // 7
            "billing_info": { // 8
                "card_number": "1234567890123456", // 9
                "expiry_date": "12/25", // 10
                "invoices": [ // 11
                    {
                        "amount": 100.0, // 12
                        "due_date": "2023-06-30" // 13
                    },
                    {
                        "amount": 100.0, // 14
                        "due_date": "2023-06-30" // 15
                    },
                ],
                "cvv": "123" // 16
            },
            "currency": "USD", // 17
            "currency_symbol": "$", // 18
        });

        // Test case 1: Get root object (line 0)
        {
            let (obj_ref, index) = get_nested_object_to_insert_into(0, &mut value);
            assert!(obj_ref.is_some());
            let obj = obj_ref.unwrap();
            assert_eq!(index, 0);
            assert!(obj.as_object().unwrap().contains_key("name"));
        }

        // Test case 2: Get the address object (line 2)
        {
            let (obj_ref, index) = get_nested_object_to_insert_into(2, &mut value);
            assert!(obj_ref.is_some());
            let obj = obj_ref.unwrap();
            assert_eq!(index, 2);
            assert!(obj.as_object().unwrap().contains_key("address"));
        }

        // Test case 3: Get the billing_info object (line 8)
        {
            let (obj_ref, index) = get_nested_object_to_insert_into(8, &mut value);
            assert!(obj_ref.is_some());
            let obj = obj_ref.unwrap();
            assert_eq!(index, 4);
            assert!(obj.as_object().unwrap().contains_key("billing_info"));
        }

        // Test case 4: Get the invoices array (line 11)
        {
            let (obj_ref, index) = get_nested_object_to_insert_into(11, &mut value);
            assert!(obj_ref.is_some());
            let obj = obj_ref.unwrap();
            assert_eq!(index, 2);
            assert!(obj.as_object().unwrap().contains_key("invoices"));
            assert!(obj.as_object().unwrap()["invoices"].is_array());
        }

        // Test case 5: Get first invoice object (line 12)
        {
            let (obj_ref, index) = get_nested_object_to_insert_into(12, &mut value);
            assert!(obj_ref.is_some());
            let obj = obj_ref.unwrap();
            assert_eq!(index, 0);
            assert!(obj.as_object().unwrap()["amount"].is_number());
        }

        // Test case 6: Get non-existent line (beyond range)
        {
            let (obj_ref, index) = get_nested_object_to_insert_into(20, &mut value);
            assert!(obj_ref.is_none());
            assert_eq!(index, 0);
        }
    }

    #[test]
    fn test_get_nested_object_to_insert_into_array_root() {
        let mut value = json!([
            "item1", // 0
            "item2", // 1
            { 
                "key": "value", // 2
                "nested": { // 3
                    "deep": "value" // 4
                }
            },
            [
                "nested1", // 5
                "nested2"  // 6
            ]
        ]);

        // Test case 1: Get root array (line 0)
        {
            let (obj_ref, index) = get_nested_object_to_insert_into(0, &mut value);
            assert!(obj_ref.is_some());
            let obj = obj_ref.unwrap();
            assert_eq!(index, 0);
            assert!(obj.is_array());
        }

        // Test case 2: Get object in array (line 2)
        {
            let (obj_ref, index) = get_nested_object_to_insert_into(2, &mut value);
            assert!(obj_ref.is_some());
            let obj = obj_ref.unwrap();
            assert_eq!(index, 0);
            assert!(obj.as_object().unwrap()["key"].is_string());
        }

        // test case 3: get nested array in array (line 4)
        {
            let (obj_ref, index) = get_nested_object_to_insert_into(4, &mut value);
            assert!(obj_ref.is_some());
            let obj = obj_ref.unwrap();
            assert_eq!(index, 0);
            assert!(obj.as_object().unwrap()["deep"].is_string());
        }
    }

    #[test]
    fn test_get_nested_object_to_insert_into_mixed_types() {
        let mut value = json!({
            "primitives": [ // 0
                123, // 1
                true, // 2
                null, // 3
                "string" // 4
            ],
            "empty": {}, // 5
            "nested_empty": { // 6
                "inner_empty": {} // 7
            }
        });

        {
            let (obj_ref, index) = get_nested_object_to_insert_into(0, &mut value);
            assert!(obj_ref.is_some());
            let obj = obj_ref.unwrap();
            assert_eq!(index, 0);
            assert!(obj.as_object().unwrap().contains_key("primitives"));
        }

        {
            let (obj_ref, index) = get_nested_object_to_insert_into(4, &mut value);
            assert!(obj_ref.is_some());
            let obj = obj_ref.unwrap();
            assert_eq!(index, 3);
            assert!(obj.is_array());
        }

        {
            let (obj_ref, index) = get_nested_object_to_insert_into(6, &mut value);
            assert!(obj_ref.is_some());
            let obj = obj_ref.unwrap();
            assert_eq!(index, 2);
            assert!(obj.as_object().unwrap().contains_key("nested_empty"));
        }

        {
            let (obj_ref, index) = get_nested_object_to_insert_into(7, &mut value);
            assert!(obj_ref.is_some());
            let obj = obj_ref.unwrap();
            assert_eq!(index, 0);
            assert!(obj.as_object().unwrap().contains_key("inner_empty"));
        }
    }
}