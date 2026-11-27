package main

import (
	"fmt"
	"maps"
	"slices"
	"strings"

	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/reflect/protodesc"
	"google.golang.org/protobuf/reflect/protoreflect"
	"google.golang.org/protobuf/reflect/protoregistry"
	"google.golang.org/protobuf/types/descriptorpb"
)

// prost maps google.protobuf to prost-types instead of generating it, so bolt-proto
// has no FILE_DESCRIPTOR_SET for those files and embeds them itself.
const wellKnownTypesPackage = "google.protobuf"

type reflectionFiles struct {
	packages       []protoreflect.FullName
	wellKnownFiles []protoreflect.FileDescriptor
}

func renderReflection(files *protoregistry.Files, targets []protoreflect.FileDescriptor) (string, error) {
	var code strings.Builder
	wellKnownFiles := map[string]protoreflect.FileDescriptor{}
	for _, file := range targets {
		services := file.Services()
		for i := range services.Len() {
			service := services.Get(i)
			needed := resolveReflectionFiles(files, file.Package())
			for _, wellKnown := range needed.wellKnownFiles {
				wellKnownFiles[wellKnown.Path()] = wellKnown
			}
			code.WriteString(renderReflectionFilesImpl(service, needed))
		}
	}
	for _, path := range slices.Sorted(maps.Keys(wellKnownFiles)) {
		wellKnown, err := renderWellKnownFile(wellKnownFiles[path])
		if err != nil {
			return "", fmt.Errorf("%s: %w", path, err)
		}
		code.WriteString(wellKnown)
	}
	return code.String(), nil
}

// resolveReflectionFiles works per package, not per file: a package's FILE_DESCRIPTOR_SET
// holds all of its files, so every import of every one of them has to be served too.
func resolveReflectionFiles(files *protoregistry.Files, root protoreflect.FullName) reflectionFiles {
	packages := map[protoreflect.FullName]bool{root: true}
	wellKnownFiles := map[string]protoreflect.FileDescriptor{}
	pending := []protoreflect.FullName{root}
	var visitWellKnown func(protoreflect.FileDescriptor)
	visitWellKnown = func(file protoreflect.FileDescriptor) {
		if _, seen := wellKnownFiles[file.Path()]; seen {
			return
		}
		wellKnownFiles[file.Path()] = file
		imports := file.Imports()
		for i := range imports.Len() {
			visitWellKnown(imports.Get(i).FileDescriptor)
		}
	}
	for len(pending) > 0 {
		pkg := pending[len(pending)-1]
		pending = pending[:len(pending)-1]
		files.RangeFilesByPackage(pkg, func(file protoreflect.FileDescriptor) bool {
			imports := file.Imports()
			for i := range imports.Len() {
				imported := imports.Get(i).FileDescriptor
				switch {
				case imported.Package() == wellKnownTypesPackage:
					visitWellKnown(imported)
				case !packages[imported.Package()]:
					packages[imported.Package()] = true
					pending = append(pending, imported.Package())
				}
			}
			return true
		})
	}

	others := slices.DeleteFunc(slices.Sorted(maps.Keys(packages)), func(pkg protoreflect.FullName) bool {
		return pkg == root
	})
	var wellKnown []protoreflect.FileDescriptor
	for _, path := range slices.Sorted(maps.Keys(wellKnownFiles)) {
		wellKnown = append(wellKnown, wellKnownFiles[path])
	}
	return reflectionFiles{
		packages:       append([]protoreflect.FullName{root}, others...),
		wellKnownFiles: wellKnown,
	}
}

func renderReflectionFilesImpl(service protoreflect.ServiceDescriptor, needed reflectionFiles) string {
	pkg := packagePath(service.ParentFile().Package())
	name := string(service.Name())
	server := fmt.Sprintf("%s::%s_server::%sServer", pkg, toSnakeCase(name), toUpperCamelCase(name))
	test := toSnakeCase(string(service.FullName()))

	var code strings.Builder
	code.WriteString("\ncrate::ext::impl_reflection_files! {\n")
	fmt.Fprintf(&code, "    %s, \"%s\";\n", test, service.FullName())
	fmt.Fprintf(&code, "    %s;\n", server)
	for _, pkg := range needed.packages {
		fmt.Fprintf(&code, "    crate::%s::FILE_DESCRIPTOR_SET,\n", packagePath(pkg))
	}
	for _, file := range needed.wellKnownFiles {
		fmt.Fprintf(&code, "    crate::bolt_ext::%s,\n", wellKnownConst(file))
	}
	code.WriteString("}\n")
	return code.String()
}

func renderWellKnownFile(file protoreflect.FileDescriptor) (string, error) {
	set := &descriptorpb.FileDescriptorSet{File: []*descriptorpb.FileDescriptorProto{protodesc.ToFileDescriptorProto(file)}}
	encoded, err := proto.MarshalOptions{Deterministic: true}.Marshal(set)
	if err != nil {
		return "", err
	}
	var code strings.Builder
	fmt.Fprintf(&code, "\n#[cfg(feature = \"grpc\")]\nconst %s: &[u8] = &[", wellKnownConst(file))
	for i, b := range encoded {
		if i%16 == 0 {
			code.WriteString("\n   ")
		}
		fmt.Fprintf(&code, " 0x%02x,", b)
	}
	code.WriteString("\n];\n")
	return code.String(), nil
}

func wellKnownConst(file protoreflect.FileDescriptor) string {
	return strings.ToUpper(strings.NewReplacer("/", "_", ".", "_").Replace(strings.TrimSuffix(file.Path(), ".proto")))
}

func packagePath(pkg protoreflect.FullName) string {
	var segments []string
	for _, part := range strings.Split(string(pkg), ".") {
		segments = append(segments, sanitizeIdentifier(toSnakeCase(part)))
	}
	return strings.Join(segments, "::")
}
