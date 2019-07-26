use failure::Error;
use krpc_protocol::{
    KRPCError,
    Message,
    MessageType,
    NodeInfo,
    Query,
    Response,
};
use serde_bencode;
use std::{
    net::SocketAddrV4,
    str::{
        self,
        FromStr,
    },
};

fn test_serialize_deserialize(parsed: Message, raw: &[u8]) -> Result<(), Error> {
    let serialized = serde_bencode::ser::to_string(&parsed)?;
    let raw_string = str::from_utf8(raw)?.to_string();

    assert_eq!(raw_string, serialized);
    assert_eq!(parsed, serde_bencode::de::from_bytes(raw)?);

    Ok(())
}

#[test]
fn ping_request() -> Result<(), Error> {
    let parsed = Message {
        ip: None,
        transaction_id: b"aa".to_vec(),
        version: None,
        message_type: MessageType::Query {
            query: Query::Ping {
                id: b"abcdefghij0123456789".into(),
            },
        },
        read_only: false,
    };

    let raw = b"d1:ad2:id20:abcdefghij0123456789e1:q4:ping1:t2:aa1:y1:qe";
    test_serialize_deserialize(parsed, raw)
}

#[test]
fn ping_read_only() -> Result<(), Error> {
    let parsed = Message {
        ip: None,
        transaction_id: b"aa".to_vec(),
        version: None,
        message_type: MessageType::Query {
            query: Query::Ping {
                id: b"abcdefghij0123456789".into(),
            },
        },
        read_only: true,
    };

    let raw = b"d1:ad2:id20:abcdefghij0123456789e1:q4:ping2:roi1e1:t2:aa1:y1:qe";
    test_serialize_deserialize(parsed, raw)
}

#[test]
fn ping_response() -> Result<(), Error> {
    let parsed = Message {
        ip: None,
        transaction_id: b"aa".to_vec(),
        version: None,
        message_type: MessageType::Response {
            response: Response::OnlyID {
                id: b"mnopqrstuvwxyz123456".into(),
            },
        },
        read_only: false,
    };

    let raw = b"d1:rd2:id20:mnopqrstuvwxyz123456e1:t2:aa1:y1:re";
    test_serialize_deserialize(parsed, raw)
}

#[test]
fn error() -> Result<(), Error> {
    let parsed = Message {
        ip: None,
        transaction_id: b"aa".to_vec(),
        version: None,
        message_type: MessageType::Error {
            error: KRPCError::new(201, "A Generic Error Ocurred"),
        },
        read_only: false,
    };

    let raw = b"d1:eli201e23:A Generic Error Ocurrede1:t2:aa1:y1:ee";
    test_serialize_deserialize(parsed, raw)
}

#[test]
fn announce_peer_request() -> Result<(), Error> {
    let parsed = Message {
        ip: None,
        transaction_id: b"aa".to_vec(),
        version: None,
        message_type: MessageType::Query {
            query: Query::AnnouncePeer {
                id: b"abcdefghij0123456789".into(),
                implied_port: true,
                port: Some(6881),
                info_hash: b"mnopqrstuvwxyz123456".into(),
                token: b"aoeusnth".to_vec(),
            },
        },
        read_only: false,
    };

    let raw = b"d1:ad2:id20:abcdefghij012345678912:implied_porti1e9:info_hash20:mnopqrstuvwxyz1234564:porti6881e5:token8:aoeusnthe1:q13:announce_peer1:t2:aa1:y1:qe";
    test_serialize_deserialize(parsed, raw)
}

#[test]
fn get_nodes_response() -> Result<(), Error> {
    let parsed = Message {
        ip: None,
        transaction_id: b"aa".to_vec(),
        version: None,
        message_type: MessageType::Response {
            response: Response::NextHop {
                id: b"abcdefghij0123456789".into(),
                token: None,
                nodes: Vec::new(),
            },
        },
        read_only: false,
    };

    let serialized = serde_bencode::ser::to_bytes(&parsed)?;
    let decoded = Message::decode(&serialized)?;

    assert_eq!(parsed, decoded);

    Ok(())
}

#[test]
fn get_nodes_response_decode() -> Result<(), Error> {
    let encoded: &[u8] = &[
        100, 50, 58, 105, 112, 54, 58, 129, 21, 63, 170, 133, 190, 49, 58, 114, 100, 50, 58, 105,
        100, 50, 48, 58, 50, 245, 78, 105, 115, 81, 255, 74, 236, 41, 205, 186, 171, 242, 251, 227,
        70, 124, 194, 103, 53, 58, 110, 111, 100, 101, 115, 52, 49, 54, 58, 48, 33, 11, 23, 67, 40,
        27, 83, 194, 152, 189, 83, 184, 116, 44, 224, 100, 119, 227, 172, 180, 211, 234, 53, 5,
        136, 247, 55, 4, 69, 93, 133, 57, 156, 104, 27, 0, 231, 29, 145, 49, 172, 172, 170, 50, 51,
        36, 37, 147, 240, 49, 120, 205, 9, 249, 147, 103, 202, 47, 147, 118, 247, 56, 14, 110, 23,
        186, 53, 174, 165, 170, 186, 95, 24, 216, 93, 124, 7, 192, 112, 119, 16, 106, 92, 58, 112,
        137, 128, 138, 141, 79, 23, 69, 24, 183, 4, 85, 166, 93, 172, 43, 127, 90, 117, 12, 129,
        47, 223, 197, 10, 15, 183, 213, 97, 35, 240, 235, 237, 50, 252, 249, 194, 225, 219, 70,
        124, 69, 205, 196, 145, 102, 100, 250, 166, 128, 104, 68, 91, 140, 182, 54, 54, 90, 21, 2,
        241, 200, 141, 23, 37, 46, 153, 74, 174, 251, 147, 165, 79, 20, 85, 75, 125, 77, 206, 96,
        25, 32, 99, 225, 224, 103, 85, 243, 146, 250, 181, 81, 97, 116, 190, 26, 225, 222, 157,
        234, 191, 56, 113, 115, 126, 188, 27, 149, 83, 240, 151, 53, 226, 74, 241, 83, 226, 84,
        251, 160, 222, 188, 171, 86, 40, 168, 238, 141, 18, 184, 130, 83, 38, 118, 45, 28, 54, 40,
        41, 156, 202, 216, 46, 98, 13, 2, 205, 26, 225, 63, 156, 12, 215, 19, 180, 67, 243, 186,
        19, 109, 221, 5, 80, 152, 247, 35, 243, 248, 56, 42, 98, 51, 123, 36, 88, 116, 101, 114,
        42, 208, 241, 77, 164, 158, 29, 72, 206, 241, 52, 116, 105, 188, 110, 109, 117, 79, 114,
        47, 76, 250, 186, 139, 146, 146, 178, 247, 93, 18, 119, 32, 235, 205, 138, 254, 102, 191,
        165, 12, 42, 220, 127, 2, 87, 195, 123, 244, 241, 208, 251, 133, 56, 218, 180, 25, 130, 48,
        88, 121, 190, 163, 198, 23, 107, 74, 12, 187, 222, 49, 70, 2, 154, 62, 129, 127, 66, 65,
        164, 135, 151, 240, 82, 208, 230, 231, 249, 209, 128, 98, 123, 231, 28, 218, 245, 70, 55,
        32, 213, 70, 20, 52, 38, 230, 211, 179, 139, 75, 33, 144, 222, 204, 108, 131, 204, 243,
        102, 133, 52, 64, 145, 124, 77, 137, 19, 62, 129, 9, 0, 237, 24, 24, 39, 3, 64, 227, 246,
        41, 203, 19, 170, 174, 98, 102, 66, 33, 245, 119, 237, 152, 161, 26, 234, 101, 49, 58, 116,
        52, 58, 0, 0, 175, 218, 49, 58, 121, 49, 58, 114, 101,
    ];

    let expected = Message {
        ip: Some("129.21.63.170:34238".parse()?),
        transaction_id: vec![0x00, 0x00, 0xAF, 0xDA],
        version: None,
        message_type: MessageType::Response {
            response: Response::NextHop {
                id: b"32f54e697351ff4aec29cdbaabf2fbe3467cc267".into(),
                token: None,
                nodes: vec![
                    NodeInfo::new(
                        b"30210b1743281b53c298bd53b8742ce06477e3ac".into(),
                        "180.211.234.53:1416".parse()?,
                    ),
                    NodeInfo::new(
                        b"f73704455d85399c681b00e71d9131acacaa3233".into(),
                        "36.37.147.240:12664".parse()?,
                    ),
                    NodeInfo::new(
                        b"cd09f99367ca2f9376f7380e6e17ba35aea5aaba".into(),
                        "95.24.216.93:31751".parse()?,
                    ),
                    NodeInfo::new(
                        b"c07077106a5c3a7089808a8d4f174518b70455a6".into(),
                        "93.172.43.127:23157".parse()?,
                    ),
                    NodeInfo::new(
                        b"0c812fdfc50a0fb7d56123f0ebed32fcf9c2e1db".into(),
                        "70.124.69.205:50321".parse()?,
                    ),
                    NodeInfo::new(
                        b"6664faa68068445b8cb636365a1502f1c88d1725".into(),
                        "46.153.74.174:64403".parse()?,
                    ),
                    NodeInfo::new(
                        b"a54f14554b7d4dce60192063e1e06755f392fab5".into(),
                        "81.97.116.190:6881".parse()?,
                    ),
                    NodeInfo::new(
                        b"de9deabf3871737ebc1b9553f09735e24af153e2".into(),
                        "84.251.160.222:48299".parse()?,
                    ),
                    NodeInfo::new(
                        b"5628a8ee8d12b8825326762d1c3628299ccad82e".into(),
                        "98.13.2.205:6881".parse()?,
                    ),
                    NodeInfo::new(
                        b"3f9c0cd713b443f3ba136ddd055098f723f3f838".into(),
                        "42.98.51.123:9304".parse()?,
                    ),
                    NodeInfo::new(
                        b"7465722ad0f14da49e1d48cef1347469bc6e6d75".into(),
                        "79.114.47.76:64186".parse()?,
                    ),
                    NodeInfo::new(
                        b"8b9292b2f75d127720ebcd8afe66bfa50c2adc7f".into(),
                        "2.87.195.123:62705".parse()?,
                    ),
                    NodeInfo::new(
                        b"d0fb8538dab41982305879bea3c6176b4a0cbbde".into(),
                        "49.70.2.154:16001".parse()?,
                    ),
                    NodeInfo::new(
                        b"7f4241a48797f052d0e6e7f9d180627be71cdaf5".into(),
                        "70.55.32.213:17940".parse()?,
                    ),
                    NodeInfo::new(
                        b"3426e6d3b38b4b2190decc6c83ccf36685344091".into(),
                        "124.77.137.19:16001".parse()?,
                    ),
                    NodeInfo::new(
                        b"0900ed1818270340e3f629cb13aaae62664221f5".into(),
                        "119.237.152.161:6890".parse()?,
                    ),
                ],
            },
        },
        read_only: false,
    };

    let message = Message::decode(encoded)?;

    assert_eq!(message, expected);

    Ok(())
}

#[test]
fn with_version() -> Result<(), Error> {
    let encoded: &[u8] = &[
        100, 50, 58, 105, 112, 54, 58, 129, 21, 60, 68, 133, 206, 49, 58, 114, 100, 50, 58, 105,
        100, 50, 48, 58, 189, 93, 60, 187, 233, 235, 179, 166, 219, 60, 135, 12, 62, 153, 36, 94,
        13, 28, 6, 241, 101, 49, 58, 116, 52, 58, 0, 0, 138, 186, 49, 58, 118, 52, 58, 85, 84, 174,
        88, 49, 58, 121, 49, 58, 114, 101,
    ];

    let expected = Message {
        ip: Some(SocketAddrV4::from_str("129.21.60.68:34254")?.into()),
        transaction_id: vec![0, 0, 138, 186],
        version: Some(vec![85, 84, 174, 88].into()),
        message_type: MessageType::Response {
            response: Response::OnlyID {
                id: b"bd5d3cbbe9ebb3a6db3c870c3e99245e0d1c06f1".into(),
            },
        },
        read_only: false,
    };

    let message = Message::decode(encoded)?;

    assert_eq!(message, expected);

    Ok(())
}
