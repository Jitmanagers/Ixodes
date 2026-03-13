use winreg::{RegValue, enums::*, types::FromRegValue};

pub fn format_reg_value(value: &RegValue) -> String {
    match value.vtype {
        REG_SZ | REG_EXPAND_SZ => {
            String::from_reg_value(value).unwrap_or_else(|_| String::from_utf8_lossy(&value.bytes).to_string())
        }
        REG_MULTI_SZ => {
            let parts: Vec<String> = Vec::from_reg_value(value).unwrap_or_default();
            parts.join(", ")
        }
        REG_DWORD => {
            let val: u32 = u32::from_reg_value(value).unwrap_or_default();
            val.to_string()
        }
        REG_QWORD => {
            let val: u64 = u64::from_reg_value(value).unwrap_or_default();
            val.to_string()
        }
        _ => {
            let text = String::from_utf8_lossy(&value.bytes)
                .trim_end_matches('\0')
                .to_string();

            if text.is_empty() || text.chars().any(|c| c.is_control() && c != '\r' && c != '\n' && c != '\t') {
                format!("hex:{}", bytes_to_hex(&value.bytes))
            } else {
                text
            }
        }
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<Vec<_>>()
        .join("")
}
