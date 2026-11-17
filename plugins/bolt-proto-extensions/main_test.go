package main

import (
	"strings"
	"testing"

	"google.golang.org/protobuf/encoding/protowire"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/reflect/protodesc"
	"google.golang.org/protobuf/reflect/protoreflect"
	"google.golang.org/protobuf/types/descriptorpb"
)

func pairOption(base, quote string) *descriptorpb.MessageOptions {
	var spec []byte
	spec = protowire.AppendTag(spec, 1, protowire.BytesType)
	spec = protowire.AppendString(spec, base)
	spec = protowire.AppendTag(spec, 2, protowire.BytesType)
	spec = protowire.AppendString(spec, quote)
	var raw []byte
	raw = protowire.AppendTag(raw, 51000, protowire.BytesType)
	raw = protowire.AppendBytes(raw, spec)
	opts := &descriptorpb.MessageOptions{}
	opts.ProtoReflect().SetUnknown(raw)
	return opts
}

func validateOption(rules ...string) *descriptorpb.FieldOptions {
	var raw []byte
	for _, rule := range rules {
		raw = protowire.AppendTag(raw, 51001, protowire.BytesType)
		raw = protowire.AppendString(raw, rule)
	}
	opts := &descriptorpb.FieldOptions{}
	opts.ProtoReflect().SetUnknown(raw)
	return opts
}

func field(name string, number int32, typ descriptorpb.FieldDescriptorProto_Type) *descriptorpb.FieldDescriptorProto {
	return &descriptorpb.FieldDescriptorProto{
		Name:     proto.String(name),
		JsonName: proto.String(name),
		Number:   proto.Int32(number),
		Label:    descriptorpb.FieldDescriptorProto_LABEL_OPTIONAL.Enum(),
		Type:     typ.Enum(),
	}
}

func optional(f *descriptorpb.FieldDescriptorProto, oneof int32) *descriptorpb.FieldDescriptorProto {
	f.Proto3Optional = proto.Bool(true)
	f.OneofIndex = proto.Int32(oneof)
	return f
}

func fixture(t *testing.T) (protoreflect.FileDescriptor, protoreflect.ExtensionDescriptor, protoreflect.ExtensionDescriptor) {
	t.Helper()
	str := descriptorpb.FieldDescriptorProto_TYPE_STRING
	options := &descriptorpb.FileDescriptorProto{
		Name:       proto.String("bolt/options/v1/options.proto"),
		Package:    proto.String("bolt.options.v1"),
		Syntax:     proto.String("proto3"),
		Dependency: []string{"google/protobuf/descriptor.proto"},
		MessageType: []*descriptorpb.DescriptorProto{{
			Name:  proto.String("PairSpec"),
			Field: []*descriptorpb.FieldDescriptorProto{field("base", 1, str), field("quote", 2, str)},
		}},
		Extension: []*descriptorpb.FieldDescriptorProto{
			func() *descriptorpb.FieldDescriptorProto {
				f := field("pair", 51000, descriptorpb.FieldDescriptorProto_TYPE_MESSAGE)
				f.TypeName = proto.String(".bolt.options.v1.PairSpec")
				f.Extendee = proto.String(".google.protobuf.MessageOptions")
				return f
			}(),
			func() *descriptorpb.FieldDescriptorProto {
				f := field("validate", 51001, str)
				f.Label = descriptorpb.FieldDescriptorProto_LABEL_REPEATED.Enum()
				f.Extendee = proto.String(".google.protobuf.FieldOptions")
				return f
			}(),
		},
	}

	leg := func() *descriptorpb.FieldDescriptorProto {
		f := field("leg", 1, descriptorpb.FieldDescriptorProto_TYPE_MESSAGE)
		f.TypeName = proto.String(".bolt.test.v1.Leg")
		return f
	}()
	typ := optional(field("type", 1, str), 0)
	typ.Options = validateOption(`r#type.as_deref() != Some("\"{}\\")`, "a\r\nb")
	count := field("count", 3, descriptorpb.FieldDescriptorProto_TYPE_UINT32)
	count.Options = validateOption("count > 0")

	test := &descriptorpb.FileDescriptorProto{
		Name:       proto.String("bolt/test/v1/test.proto"),
		Package:    proto.String("bolt.test.v1"),
		Syntax:     proto.String("proto3"),
		Dependency: []string{"bolt/options/v1/options.proto"},
		MessageType: []*descriptorpb.DescriptorProto{
			{
				Name:  proto.String("Leg"),
				Field: []*descriptorpb.FieldDescriptorProto{field("asset", 1, str)},
			},
			{
				Name:    proto.String("Flat"),
				Field:   []*descriptorpb.FieldDescriptorProto{field("base", 1, str), field("quote", 2, descriptorpb.FieldDescriptorProto_TYPE_UINT64)},
				Options: pairOption("base", "quote"),
			},
			{
				Name: proto.String("Optionals"),
				Field: []*descriptorpb.FieldDescriptorProto{
					typ,
					optional(field("quote", 2, descriptorpb.FieldDescriptorProto_TYPE_BYTES), 1),
					count,
				},
				OneofDecl: []*descriptorpb.OneofDescriptorProto{{Name: proto.String("_type")}, {Name: proto.String("_quote")}},
				Options:   pairOption("type", "quote"),
			},
			{
				Name:    proto.String("Deep"),
				Field:   []*descriptorpb.FieldDescriptorProto{leg, field("n", 2, descriptorpb.FieldDescriptorProto_TYPE_SINT64)},
				Options: pairOption("leg.asset", "n"),
			},
		},
	}

	files, err := protodesc.NewFiles(&descriptorpb.FileDescriptorSet{File: []*descriptorpb.FileDescriptorProto{
		protodesc.ToFileDescriptorProto(descriptorpb.File_google_protobuf_descriptor_proto),
		options,
		test,
	}})
	if err != nil {
		t.Fatal(err)
	}
	pair, err := findExtension(files, pairExtension)
	if err != nil || pair == nil {
		t.Fatalf("pair extension: %v", err)
	}
	validate, err := findExtension(files, validateExtension)
	if err != nil || validate == nil {
		t.Fatalf("validate extension: %v", err)
	}
	file, err := files.FindFileByPath("bolt/test/v1/test.proto")
	if err != nil {
		t.Fatal(err)
	}
	return file, pair, validate
}

func TestRenderMessage(t *testing.T) {
	file, pairExt, validateExt := fixture(t)
	var got strings.Builder
	for _, message := range allMessages(file.Messages()) {
		code, err := renderMessage(message, pairExt, validateExt)
		if err != nil {
			t.Fatalf("%s: %v", message.FullName(), err)
		}
		got.WriteString(code)
	}

	want := `
crate::ext::impl_pair! {
    crate::bolt::test::v1::Flat;
    paths = "base", "quote";
    infallible;
    base: [] base: string req;
    quote: [] quote: uint64 req;
}

crate::ext::impl_pair! {
    crate::bolt::test::v1::Optionals;
    paths = "type", "quote";
    fallible;
    base: [] r#type: string opt;
    quote: [] quote: bytes opt;
}

crate::ext::impl_validate! {
    crate::bolt::test::v1::Optionals, "bolt.test.v1.Optionals", self;
    r#type ref "type" {
        "r#type.as_deref() != Some(\"\\\"{}\\\\\")" => r#type.as_deref() != Some("\"{}\\"),
        "a\r\nb" => a` + "\r\n" + `b,
    }
    count val "count" {
        "count > 0" => count > 0,
    }
}

crate::ext::impl_pair! {
    crate::bolt::test::v1::Deep;
    paths = "leg.asset", "n";
    fallible;
    base: [leg] asset: string req;
    quote: [] n: sint64 req;
}
`
	if got.String() != want {
		t.Errorf("got:\n%s\nwant:\n%s", got.String(), want)
	}
}
