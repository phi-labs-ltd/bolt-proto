use prost::encoding::{encode_key, encode_varint, WireType};
use prost::Message;
use prost_types::field_descriptor_proto::{Label, Type};
use prost_types::{
    DescriptorProto, FieldDescriptorProto, FileDescriptorProto, OneofDescriptorProto,
};

use super::*;

const TEST_FILE: &str = "bolt/test/v1/test.proto";

fn len_field(tag: u32, bytes: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    encode_key(tag, WireType::LengthDelimited, &mut buf);
    encode_varint(bytes.len() as u64, &mut buf);
    buf.extend_from_slice(bytes);
    buf
}

fn field(name: &str, number: i32, ty: Type) -> FieldDescriptorProto {
    FieldDescriptorProto {
        name: Some(name.to_owned()),
        json_name: Some(name.to_owned()),
        number: Some(number),
        label: Some(Label::Optional as i32),
        r#type: Some(ty as i32),
        ..Default::default()
    }
}

fn message_field(name: &str, number: i32, type_name: &str) -> FieldDescriptorProto {
    FieldDescriptorProto {
        type_name: Some(type_name.to_owned()),
        ..field(name, number, Type::Message)
    }
}

fn message(name: &str, fields: Vec<FieldDescriptorProto>) -> DescriptorProto {
    DescriptorProto {
        name: Some(name.to_owned()),
        field: fields,
        ..Default::default()
    }
}

/// `DescriptorProto` bytes with `option (bolt.options.v1.id) = {fields: [..]}` appended;
/// `prost_types::MessageOptions` has no slot for extensions, so they go on as raw bytes.
fn with_id(message: DescriptorProto, paths: &[&str]) -> Vec<u8> {
    let spec: Vec<u8> = paths
        .iter()
        .flat_map(|p| len_field(1, p.as_bytes()))
        .collect();
    let options = len_field(51000, &spec);
    let mut bytes = message.encode_to_vec();
    bytes.extend(len_field(7, &options));
    bytes
}

fn options_file() -> FileDescriptorProto {
    FileDescriptorProto {
        name: Some("bolt/options/v1/options.proto".to_owned()),
        package: Some("bolt.options.v1".to_owned()),
        dependency: vec!["google/protobuf/descriptor.proto".to_owned()],
        message_type: vec![message(
            "IdSpec",
            vec![FieldDescriptorProto {
                label: Some(Label::Repeated as i32),
                ..field("fields", 1, Type::String)
            }],
        )],
        extension: vec![FieldDescriptorProto {
            extendee: Some(".google.protobuf.MessageOptions".to_owned()),
            ..message_field("id", 51000, ".bolt.options.v1.IdSpec")
        }],
        syntax: Some("proto3".to_owned()),
        ..Default::default()
    }
}

fn assets_file() -> FileDescriptorProto {
    FileDescriptorProto {
        name: Some("bolt/assets/v1/assets.proto".to_owned()),
        package: Some("bolt.assets.v1".to_owned()),
        message_type: vec![message(
            "Pair",
            vec![
                field("base", 1, Type::String),
                field("quote", 2, Type::String),
            ],
        )],
        syntax: Some("proto3".to_owned()),
        ..Default::default()
    }
}

/// Builds a pool of descriptor.proto, options.proto, assets.proto (`Pair`) and a
/// test file holding `plain` messages plus `with_ids` messages (already encoded).
fn pool(plain: Vec<DescriptorProto>, with_ids: Vec<Vec<u8>>) -> DescriptorPool {
    let test_file = FileDescriptorProto {
        name: Some(TEST_FILE.to_owned()),
        package: Some("bolt.test.v1".to_owned()),
        dependency: vec![
            "bolt/options/v1/options.proto".to_owned(),
            "bolt/assets/v1/assets.proto".to_owned(),
        ],
        message_type: plain,
        syntax: Some("proto3".to_owned()),
        ..Default::default()
    };
    let mut test_bytes = test_file.encode_to_vec();
    for m in &with_ids {
        test_bytes.extend(len_field(4, m));
    }

    let descriptor_proto = DescriptorPool::global()
        .get_file_by_name("google/protobuf/descriptor.proto")
        .expect("global pool has descriptor.proto")
        .encode_to_vec();
    let files = [
        descriptor_proto,
        options_file().encode_to_vec(),
        assets_file().encode_to_vec(),
        test_bytes,
    ];
    let set: Vec<u8> = files.iter().flat_map(|f| len_field(1, f)).collect();
    DescriptorPool::decode(set.as_slice()).expect("valid descriptor set")
}

fn run(plain: Vec<DescriptorProto>, with_ids: Vec<Vec<u8>>) -> Result<String, String> {
    generate(&pool(plain, with_ids), &[TEST_FILE.to_owned()])
}

#[test]
fn nested_paths_unwrap_the_message_field_once() {
    let hedged = message(
        "HedgedExposure",
        vec![message_field("instrument", 1, ".bolt.assets.v1.Pair")],
    );
    let out = run(
        vec![],
        vec![with_id(hedged, &["instrument.base", "instrument.quote"])],
    )
    .unwrap();
    assert!(
        out.contains("impl crate::bolt::test::v1::HedgedExposure {"),
        "{out}"
    );
    assert!(
        out.contains("pub fn id(&self) -> ::core::option::Option<(&str, &str)> {"),
        "{out}"
    );
    assert!(out.contains("let m0 = self.instrument.as_ref()?;"), "{out}");
    assert_eq!(out.matches("as_ref()?").count(), 1, "{out}");
    assert!(
        out.contains("Some((m0.base.as_str(), m0.quote.as_str()))"),
        "{out}"
    );
}

#[test]
fn id_picks_one_of_several_pairs() {
    let legs = message(
        "Legs",
        vec![
            message_field("leg_a", 1, ".bolt.assets.v1.Pair"),
            message_field("leg_b", 2, ".bolt.assets.v1.Pair"),
        ],
    );
    let out = run(vec![], vec![with_id(legs, &["leg_b.base", "leg_b.quote"])]).unwrap();
    assert!(out.contains("let m0 = self.leg_b.as_ref()?;"), "{out}");
    assert!(!out.contains("leg_a"), "{out}");
}

#[test]
fn top_level_scalars_return_a_bare_value() {
    let venue = message(
        "Venue",
        vec![
            field("name", 1, Type::String),
            field("block_height", 2, Type::Uint64),
        ],
    );
    let out = run(
        vec![],
        vec![with_id(venue.clone(), &["name", "block_height"])],
    )
    .unwrap();
    assert!(out.contains("pub fn id(&self) -> (&str, u64) {"), "{out}");
    assert!(
        out.contains("(self.name.as_str(), self.block_height)"),
        "{out}"
    );

    let out = run(vec![], vec![with_id(venue, &["block_height"])]).unwrap();
    assert!(out.contains("pub fn id(&self) -> u64 {"), "{out}");
}

#[test]
fn proto3_optional_leaf_makes_id_fallible() {
    let msg = DescriptorProto {
        field: vec![FieldDescriptorProto {
            proto3_optional: Some(true),
            oneof_index: Some(0),
            ..field("venue", 1, Type::String)
        }],
        oneof_decl: vec![OneofDescriptorProto {
            name: Some("_venue".to_owned()),
            ..Default::default()
        }],
        ..message("Quote", vec![])
    };
    let out = run(vec![], vec![with_id(msg, &["venue"])]).unwrap();
    assert!(out.contains("-> ::core::option::Option<&str>"), "{out}");
    assert!(out.contains("Some(self.venue.as_deref()?)"), "{out}");
}

#[test]
fn messages_without_the_option_are_skipped() {
    let out = run(
        vec![message("Plain", vec![field("name", 1, Type::String)])],
        vec![],
    )
    .unwrap();
    assert_eq!(out, HEADER);
}

#[test]
fn rejects_bad_paths() {
    let hedged = || {
        message(
            "HedgedExposure",
            vec![
                message_field("instrument", 1, ".bolt.assets.v1.Pair"),
                field("note", 2, Type::String),
                FieldDescriptorProto {
                    label: Some(Label::Repeated as i32),
                    ..field("tags", 3, Type::String)
                },
            ],
        )
    };
    let cases: [(&[&str], &str); 5] = [
        (&[], "at least one field path"),
        (&["instrument.nope"], "has no field `nope`"),
        (&["note.base"], "not a message field"),
        (&["instrument"], "must be a scalar field"),
        (&["tags"], "repeated or a map"),
    ];
    for (paths, expected) in cases {
        let err = run(vec![], vec![with_id(hedged(), paths)]).unwrap_err();
        assert!(err.contains(expected), "{paths:?}: {err}");
        assert!(
            err.starts_with("bolt/test/v1/test.proto: bolt.test.v1.HedgedExposure: "),
            "{err}"
        );
    }
}

#[test]
fn rejects_collision_with_prost_getter() {
    let msg = message(
        "Order",
        vec![
            FieldDescriptorProto {
                type_name: Some(".bolt.test.v1.Status".to_owned()),
                ..field("id", 1, Type::Enum)
            },
            field("name", 2, Type::String),
        ],
    );
    let file = FileDescriptorProto {
        name: Some(TEST_FILE.to_owned()),
        package: Some("bolt.test.v1".to_owned()),
        dependency: vec!["bolt/options/v1/options.proto".to_owned()],
        enum_type: vec![prost_types::EnumDescriptorProto {
            name: Some("Status".to_owned()),
            value: vec![prost_types::EnumValueDescriptorProto {
                name: Some("STATUS_UNSPECIFIED".to_owned()),
                number: Some(0),
                ..Default::default()
            }],
            ..Default::default()
        }],
        syntax: Some("proto3".to_owned()),
        ..Default::default()
    };
    let mut file_bytes = file.encode_to_vec();
    file_bytes.extend(len_field(4, &with_id(msg, &["name"])));
    let descriptor_proto = DescriptorPool::global()
        .get_file_by_name("google/protobuf/descriptor.proto")
        .unwrap()
        .encode_to_vec();
    let set: Vec<u8> = [descriptor_proto, options_file().encode_to_vec(), file_bytes]
        .iter()
        .flat_map(|f| len_field(1, f))
        .collect();
    let pool = DescriptorPool::decode(set.as_slice()).unwrap();
    let err = generate(&pool, &[TEST_FILE.to_owned()]).unwrap_err();
    assert!(err.contains("collides with the generated `id()`"), "{err}");
}

#[test]
fn rust_paths_follow_prost_module_layout() {
    assert_eq!(module_ident("type"), "r#type");
    assert_eq!(module_ident("self"), "self_");
    assert_eq!(module_ident("inventory_feeder"), "inventory_feeder");
    assert_eq!(type_ident("Self"), "Self_");
}
