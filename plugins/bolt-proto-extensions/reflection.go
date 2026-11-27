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
	_ "google.golang.org/protobuf/types/known/anypb"
	_ "google.golang.org/protobuf/types/known/apipb"
	_ "google.golang.org/protobuf/types/known/durationpb"
	_ "google.golang.org/protobuf/types/known/emptypb"
	_ "google.golang.org/protobuf/types/known/fieldmaskpb"
	_ "google.golang.org/protobuf/types/known/sourcecontextpb"
	_ "google.golang.org/protobuf/types/known/structpb"
	_ "google.golang.org/protobuf/types/known/timestamppb"
	_ "google.golang.org/protobuf/types/known/typepb"
	_ "google.golang.org/protobuf/types/known/wrapperspb"
)

// prost maps google.protobuf to prost-types instead of generating it, so bolt-proto has no
// FILE_DESCRIPTOR_SET for those files and embeds them itself. They come from the protobuf
// Go module, not the request: buf prunes the request's copies to the types the targets
// use, and a pruned file served under its real name hides the full one from reflection
// clients that also need other parts of it.
const wellKnownTypesPackage = "google.protobuf"

type reflectionFiles struct {
	packages       []protoreflect.FullName
	wellKnownFiles []string
}

func renderReflection(files *protoregistry.Files, targets []protoreflect.FileDescriptor) (string, error) {
	var code strings.Builder
	resolved := map[protoreflect.FullName]reflectionFiles{}
	wellKnownFiles := map[string]bool{}
	var servers []string
	for _, file := range targets {
		services := file.Services()
		if services.Len() == 0 {
			continue
		}
		needed, ok := resolved[file.Package()]
		if !ok {
			var err error
			if needed, err = resolveReflectionFiles(files, file.Package()); err != nil {
				return "", fmt.Errorf("%s: %w", file.Package(), err)
			}
			resolved[file.Package()] = needed
		}
		for _, path := range needed.wellKnownFiles {
			wellKnownFiles[path] = true
		}
		for i := range services.Len() {
			server := serverPath(services.Get(i))
			servers = append(servers, server)
			code.WriteString(renderReflectionFilesImpl(server, needed))
		}
	}
	if len(servers) > 0 {
		code.WriteString("\ncrate::ext::test_reflection_files! {\n")
		for _, server := range servers {
			fmt.Fprintf(&code, "    %s,\n", server)
		}
		code.WriteString("}\n")
	}
	for _, path := range slices.Sorted(maps.Keys(wellKnownFiles)) {
		wellKnown, err := renderWellKnownFile(path)
		if err != nil {
			return "", fmt.Errorf("%s: %w", path, err)
		}
		code.WriteString(wellKnown)
	}
	return code.String(), nil
}

// resolveReflectionFiles works per package, not per file: a package's FILE_DESCRIPTOR_SET
// holds all of its files, so every import of every one of them has to be served too.
func resolveReflectionFiles(files *protoregistry.Files, root protoreflect.FullName) (reflectionFiles, error) {
	packages := map[protoreflect.FullName]bool{root: true}
	wellKnownFiles := map[string]bool{}
	var visitWellKnown func(path string) error
	visitWellKnown = func(path string) error {
		if wellKnownFiles[path] {
			return nil
		}
		file, err := protoregistry.GlobalFiles.FindFileByPath(path)
		if err != nil {
			return fmt.Errorf("%s is not linked into bolt-proto-extensions: %w", path, err)
		}
		wellKnownFiles[path] = true
		imports := file.Imports()
		for i := range imports.Len() {
			if err := visitWellKnown(imports.Get(i).Path()); err != nil {
				return err
			}
		}
		return nil
	}

	var err error
	pending := []protoreflect.FullName{root}
	for len(pending) > 0 && err == nil {
		pkg := pending[len(pending)-1]
		pending = pending[:len(pending)-1]
		files.RangeFilesByPackage(pkg, func(file protoreflect.FileDescriptor) bool {
			imports := file.Imports()
			for i := range imports.Len() {
				imported := imports.Get(i).FileDescriptor
				switch {
				case imported.Package() == wellKnownTypesPackage:
					if err = visitWellKnown(imported.Path()); err != nil {
						return false
					}
				case !packages[imported.Package()]:
					packages[imported.Package()] = true
					pending = append(pending, imported.Package())
				}
			}
			return true
		})
	}
	if err != nil {
		return reflectionFiles{}, err
	}

	others := slices.DeleteFunc(slices.Sorted(maps.Keys(packages)), func(pkg protoreflect.FullName) bool {
		return pkg == root
	})
	return reflectionFiles{
		packages:       append([]protoreflect.FullName{root}, others...),
		wellKnownFiles: slices.Sorted(maps.Keys(wellKnownFiles)),
	}, nil
}

func serverPath(service protoreflect.ServiceDescriptor) string {
	name := string(service.Name())
	return fmt.Sprintf("%s::%s_server::%sServer", packagePath(service.ParentFile().Package()), toSnakeCase(name), toUpperCamelCase(name))
}

func renderReflectionFilesImpl(server string, needed reflectionFiles) string {
	var code strings.Builder
	code.WriteString("\ncrate::ext::impl_reflection_files! {\n")
	fmt.Fprintf(&code, "    %s;\n", server)
	for _, pkg := range needed.packages {
		fmt.Fprintf(&code, "    crate::%s::FILE_DESCRIPTOR_SET,\n", packagePath(pkg))
	}
	for _, path := range needed.wellKnownFiles {
		fmt.Fprintf(&code, "    crate::bolt_ext::%s,\n", wellKnownConst(path))
	}
	code.WriteString("}\n")
	return code.String()
}

func renderWellKnownFile(path string) (string, error) {
	file, err := protoregistry.GlobalFiles.FindFileByPath(path)
	if err != nil {
		return "", err
	}
	descriptor := protodesc.ToFileDescriptorProto(file)
	descriptor.SourceCodeInfo = nil
	set := &descriptorpb.FileDescriptorSet{File: []*descriptorpb.FileDescriptorProto{descriptor}}
	encoded, err := proto.MarshalOptions{Deterministic: true}.Marshal(set)
	if err != nil {
		return "", err
	}
	var code strings.Builder
	fmt.Fprintf(&code, "\n#[cfg(feature = \"grpc\")]\nconst %s: &[u8] = &[", wellKnownConst(path))
	for i, b := range encoded {
		if i%16 == 0 {
			code.WriteString("\n   ")
		}
		fmt.Fprintf(&code, " 0x%02x,", b)
	}
	code.WriteString("\n];\n")
	return code.String(), nil
}

func wellKnownConst(path string) string {
	return strings.ToUpper(strings.NewReplacer("/", "_", ".", "_").Replace(strings.TrimSuffix(path, ".proto")))
}

func packagePath(pkg protoreflect.FullName) string {
	var segments []string
	for _, part := range strings.Split(string(pkg), ".") {
		segments = append(segments, sanitizeIdentifier(toSnakeCase(part)))
	}
	return strings.Join(segments, "::")
}
