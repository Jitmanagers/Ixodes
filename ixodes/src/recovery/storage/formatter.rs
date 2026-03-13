use serde::Serialize;
use serde_json::Value;

pub trait ArtifactFormatter {
    fn is_valuable(&self) -> bool;
    fn to_json_bytes(&self) -> Result<Vec<u8>, serde_json::Error>;
}

impl<T: Serialize> ArtifactFormatter for T {
    fn is_valuable(&self) -> bool {
        if let Ok(value) = serde_json::to_value(self) {
            match value {
                Value::Array(v) => !v.is_empty(),
                Value::Object(m) => {
                    if m.is_empty() {
                        return false;
                    }

                    for (_, val) in m.iter() {
                        match val {
                            Value::Array(arr) => {
                                if !arr.is_empty() {
                                    return true;
                                }
                            }
                            Value::Object(obj) => {
                                if !obj.is_empty() {
                                    return true;
                                }
                            }
                            Value::String(s) => {
                                if !s.is_empty() {
                                    return true;
                                }
                            }
                            Value::Number(_) | Value::Bool(_) => return true,
                            Value::Null => continue,
                        }
                    }
                    false
                }
                Value::String(s) => !s.trim().is_empty(),
                Value::Null => false,
                _ => true,
            }
        } else {
            true
        }
    }

    fn to_json_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec_pretty(self)
    }
}
