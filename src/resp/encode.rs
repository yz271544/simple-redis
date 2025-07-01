use crate::{
    BulkString, RespArray, RespEncode, RespMap, RespNull, RespNullArray, RespNullBulkString,
    RespSet, SimpleError, SimpleString,
};

/*
- 如何解析 Frame
  - simple string: "+OK\r\n"
  - error: "-Error message\r\n"
  - bulk error: "!<length>\r\n<error>\r\n"
  - integer: ":[<+|->]<value>\r\n"
  - bulk string: "$<length>\r\n<string>\r\n"
  - null bulk string: "$-1\r\n"
  - array: "*<number-of-elements>\r\n<element-1>...<element-n>\r\n"

  - null array: "*-1\r\n"
  - null: "_\r\n"
  - boolean: ":<t|f>\r\n"
  - double: ",[<+|->]<integral>[.<fractional>][<E|e>[sign]<exponent>]\r\n"
  - big number: "([+|-]<number>\r\n"
  - map: "%<number-of-entries>\r\n<key-1><value-1>...<key-n><value-n>\r\n"
  - set: "~<number-of-elements>\r\n<element-1>...<element-n>\r\n"
*/

const BUF_CAP: usize = 4096;

// - integer: ":[<+|->]<value>\r\n"
impl RespEncode for i64 {
    fn encode(self) -> Vec<u8> {
        let sign = if self < 0 { "" } else { "+" };
        format!(":{}{}\r\n", sign, self).into_bytes()
    }
}
// - error: "-Error message\r\n"
impl RespEncode for SimpleError {
    fn encode(self) -> Vec<u8> {
        format!("-{}\r\n", self.0).into_bytes()
    }
}
// - simple string: "+OK\r\n"
impl RespEncode for SimpleString {
    fn encode(self) -> Vec<u8> {
        format!("+{}\r\n", self.0).into_bytes()
    }
}
// - bulk string: "$<length>\r\n<string>\r\n"
impl RespEncode for BulkString {
    fn encode(self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.len() + 16);
        buf.extend_from_slice(&format!("${}\r\n", self.len()).into_bytes());
        buf.extend_from_slice(&self.0);
        buf.extend_from_slice(b"\r\n");
        buf
    }
}

// - null bulk string: "$-1\r\n"
impl RespEncode for RespNullBulkString {
    fn encode(self) -> Vec<u8> {
        b"$-1\r\n".to_vec()
    }
}

// - array: *<number-of-elements>\r\n<element-1>...<element-n>
impl RespEncode for RespArray {
    fn encode(self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(BUF_CAP);
        buf.extend_from_slice(&format!("*{}\r\n", self.len()).into_bytes());
        for frame in self.0 {
            buf.extend_from_slice(&frame.encode());
        }
        buf
    }
}

// - null: "_\r\n"
impl RespEncode for RespNull {
    fn encode(self) -> Vec<u8> {
        b"_\r\n".to_vec()
    }
}

// - boolean: ":<t|f>\r\n"
impl RespEncode for bool {
    fn encode(self) -> Vec<u8> {
        let value = if self { "t" } else { "f" };
        format!(":{}\r\n", value).into_bytes()
    }
}

// - double: ",[<+|->]<integral>[.<fractional>][<E|e>[sign]<exponent>]\r\n"
impl RespEncode for f64 {
    fn encode(self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(32);
        let ret = if self.abs() > 1e+8 || self.abs() < 1e-8 {
            format!(",{:+e}\r\n", self)
        } else {
            let sign = if self < 0.0 { "" } else { "+" };
            format!(",{}{}\r\n", sign, self)
        };
        buf.extend_from_slice(&ret.into_bytes());
        buf
    }
}

// - null array: "*-1\r\n"
impl RespEncode for RespNullArray {
    fn encode(self) -> Vec<u8> {
        b"*-1\r\n".to_vec()
    }
}

// - map: "%<number-of-entries>\r\n<key-1><value-1>...<key-n><value-n>\r\n"
impl RespEncode for RespMap {
    fn encode(self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(BUF_CAP);
        buf.extend_from_slice(&format!("%{}\r\n", self.0.len()).into_bytes());
        for (key, value) in self.0 {
            buf.extend_from_slice(&SimpleString::new(key).encode());
            buf.extend_from_slice(&value.encode());
        }
        buf
    }
}

// - set: "~<number-of-elements>\r\n<element-1>...<element-n>\r\n"
impl RespEncode for RespSet {
    fn encode(self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(BUF_CAP);
        buf.extend_from_slice(&format!("~{}\r\n", self.0.len()).into_bytes());
        for element in self.0 {
            buf.extend_from_slice(&element.encode());
        }
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_simple_string() {
        let frame: crate::RespFrame = SimpleString::new("OK".to_string()).into();
        assert_eq!(frame.encode(), b"+OK\r\n".to_vec());
    }

    #[test]
    fn test_encode_error() {
        let frame: crate::RespFrame = SimpleError::new("Error message".to_string()).into();
        assert_eq!(frame.encode(), b"-Error message\r\n".to_vec());
    }

    #[test]
    fn test_map_encode() {
        let mut map = RespMap::new();
        map.insert(
            "hello".to_string(),
            BulkString::new("world".to_string()).into(),
        );
        map.insert("foo".to_string(), (-123456.789).into());

        let frame: crate::RespFrame = map.into();
        // assert_eq!(
        //     String::from_utf8_lossy(&frame.encode()),
        //     "%2\r\n+foo\r\n,-123456.789\r\n+hello\r\n$5\r\nworld\r\n"
        // );
        assert_eq!(
            frame.encode(),
            b"%2\r\n+foo\r\n,-123456.789\r\n+hello\r\n$5\r\nworld\r\n".to_vec()
        );
    }

    #[test]
    fn test_encode_integer() {
        let frame: crate::RespFrame = 123.into();
        assert_eq!(frame.encode(), b":+123\r\n".to_vec());

        let frame: crate::RespFrame = (-123).into();
        assert_eq!(frame.encode(), b":-123\r\n".to_vec());
    }

    #[test]
    fn test_bulk_string_encode() {
        let frame: crate::RespFrame = BulkString::new(b"hello".to_vec()).into();
        assert_eq!(frame.encode(), b"$5\r\nhello\r\n");
    }

    #[test]
    fn test_array_encode() {
        let frame: crate::RespFrame = RespArray::new(vec![
            BulkString::new("set".to_string()).into(),
            BulkString::new("hello".to_string()).into(),
            BulkString::new("world".to_string()).into(),
        ])
        .into();
        // assert_eq!(
        //     String::from_utf8_lossy(&frame.encode()),
        //     "*3\r\n$3\r\nset\r\n$5\r\nhello\r\n$5\r\nworld\r\n"
        // );
        assert_eq!(
            frame.encode(),
            b"*3\r\n$3\r\nset\r\n$5\r\nhello\r\n$5\r\nworld\r\n".to_vec()
        );
    }

    #[test]
    fn test_integer_encode() {
        let frame: crate::RespFrame = 123.into();
        // assert_eq!(String::from_utf8_lossy(&frame.encode()), ":+123\r\n");
        assert_eq!(frame.encode(), b":+123\r\n");

        let frame: crate::RespFrame = (-123).into();
        assert_eq!(frame.encode(), b":-123\r\n");
    }

    #[test]
    fn test_null_bulk_string_encode() {
        let frame: crate::RespFrame = RespNullBulkString.into();
        assert_eq!(frame.encode(), b"$-1\r\n");
    }

    #[test]
    fn test_null_array_encode() {
        let frame: crate::RespFrame = RespNullArray.into();
        assert_eq!(frame.encode(), b"*-1\r\n");
    }

    #[test]
    fn test_null_encode() {
        let frame: crate::RespFrame = RespNull.into();
        assert_eq!(frame.encode(), b"_\r\n");
    }

    #[test]
    fn test_boolean_encode() {}

    #[test]
    fn test_double_encode() {
        let frame: crate::RespFrame = 123.456.into();
        assert_eq!(frame.encode(), b",+123.456\r\n");

        let frame: crate::RespFrame = (-123.456).into();
        assert_eq!(frame.encode(), b",-123.456\r\n");

        let frame: crate::RespFrame = 1.23456e+8.into();
        //assert_eq!(String::from_utf8_lossy(&frame.encode()), ",+1.23456e8\r\n");
        assert_eq!(frame.encode(), b",+1.23456e8\r\n");

        let frame: crate::RespFrame = (-1.23456e-9).into();
        assert_eq!(frame.encode(), b",-1.23456e-9\r\n");
    }

    #[test]
    fn test_set_encode() {
        let frame: crate::RespFrame = RespSet::new([
            RespArray::new([1234.into(), true.into()]).into(),
            BulkString::new("world".to_string()).into(),
        ])
        .into();
        // assert_eq!(
        //     String::from_utf8_lossy(&frame.encode()),
        //     "~2\r\n*2\r\n:+1234\r\n:t\r\n$5\r\nworld\r\n"
        // );
        assert_eq!(
            frame.encode(),
            b"~2\r\n*2\r\n:+1234\r\n:t\r\n$5\r\nworld\r\n".to_vec()
        );
    }
}
