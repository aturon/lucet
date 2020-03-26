use lucet_module::bindings::Bindings;
use std::collections::HashMap;
use std::path::PathBuf;

fn load_wat_module(name: &str) -> Vec<u8> {
    use std::fs::File;
    use std::io::Read;
    use wabt::Wat2Wasm;
    let watfile = PathBuf::from(&format!("tests/wasm/{}.wat", name));
    let mut contents = Vec::new();
    let mut file = File::open(&watfile).expect("open module file");
    file.read_to_end(&mut contents).expect("read module file");
    Wat2Wasm::new()
        .write_debug_names(true)
        .convert(contents)
        .expect("convert module to wasm binary format")
        .as_ref()
        .to_owned()
}

pub fn test_bindings() -> Bindings {
    let imports: HashMap<String, String> = [
        ("icalltarget".into(), "icalltarget".into()), // icall_import
        ("inc".into(), "inc".into()),                 // import
        ("imp_0".into(), "imp_0".into()),             // import_many
        ("imp_1".into(), "imp_1".into()),             // import_many
        ("imp_2".into(), "imp_2".into()),             // import_many
        ("imp_3".into(), "imp_3".into()),             // import_many
        ("imported_main".into(), "imported_main".into()), // exported_import
    ]
    .iter()
    .cloned()
    .collect();

    Bindings::env(imports)
}

mod module_data {
    /// Tests of the `ModuleData` generated by the lucetc Compiler
    use super::load_wat_module;
    use lucet_module::bindings::Bindings;
    use lucetc::{Compiler, CpuFeatures, HeapSettings, OptLevel};
    use std::path::PathBuf;
    use target_lexicon::Triple;

    #[test]
    fn exported_import() {
        let m = load_wat_module("exported_import");
        let b = super::test_bindings();
        let h = HeapSettings::default();
        let c = Compiler::new(
            &m,
            Triple::host(),
            OptLevel::default(),
            CpuFeatures::default(),
            &b,
            h,
            false,
            &None,
            false,
        )
        .expect("compiling exported_import");
        let mdata = c.module_data().unwrap();
        assert_eq!(mdata.globals_spec().len(), 0);

        assert_eq!(mdata.import_functions().len(), 2);
        assert_eq!(mdata.export_functions().len(), 2);
        assert_eq!(mdata.function_info().len(), 4);
        // This ordering is actually arbitrary. Cranelift hoists additional declaration modifiers
        // up to the function declaration. This means inc comes first, and main second, in
        // `exported_import.wat`.
        assert_eq!(mdata.export_functions()[0].names, vec!["exported_inc"]);
        assert_eq!(mdata.export_functions()[1].names, vec!["exported_main"]);
    }

    #[test]
    fn multiple_import() {
        let m = load_wat_module("multiple_import");
        let b = super::test_bindings();
        let builder = Compiler::builder();
        let c = builder.create(&m, &b).expect("compiling multiple_import");
        let mdata = c.module_data().unwrap();
        assert_eq!(mdata.globals_spec().len(), 0);

        assert_eq!(mdata.import_functions().len(), 2);
        assert_eq!(mdata.export_functions().len(), 1);
        assert_eq!(mdata.function_info().len(), 4);
        assert_eq!(mdata.export_functions()[0].names, vec!["exported_inc"]);
    }

    #[test]
    fn globals_export() {
        let m = load_wat_module("globals_export");
        let b = super::test_bindings();
        let builder = Compiler::builder();
        let c = builder.create(&m, &b).expect("compiling globals_export");
        let mdata = c.module_data().unwrap();

        assert_eq!(mdata.globals_spec().len(), 1);
        assert_eq!(mdata.globals_spec()[0].export_names(), &["start", "dupe"]);

        assert_eq!(mdata.import_functions().len(), 0);
        assert_eq!(mdata.export_functions().len(), 0);
        assert_eq!(mdata.function_info().len(), 2);
    }

    #[test]
    fn fibonacci() {
        let m = load_wat_module("fibonacci");
        let b = super::test_bindings();
        let builder = Compiler::builder();
        let c = builder.create(&m, &b).expect("compiling fibonacci");
        let mdata = c.module_data().unwrap();
        assert_eq!(mdata.globals_spec().len(), 0);

        assert_eq!(mdata.import_functions().len(), 0);
        assert_eq!(mdata.function_info().len(), 3);
        assert_eq!(mdata.export_functions()[0].names, vec!["main"]);
    }

    #[test]
    fn arith() {
        let m = load_wat_module("arith");
        let b = Bindings::empty();
        let builder = Compiler::builder();
        let c = builder.create(&m, &b).expect("compiling arith");
        let mdata = c.module_data().unwrap();
        assert_eq!(mdata.globals_spec().len(), 0);

        assert_eq!(mdata.import_functions().len(), 0);
        assert_eq!(mdata.function_info().len(), 3);
        assert_eq!(mdata.export_functions()[0].names, vec!["main"]);
    }

    #[test]
    fn duplicate_imports() {
        let m = load_wat_module("duplicate_imports");
        let b = Bindings::from_file(&PathBuf::from(
            "tests/bindings/duplicate_imports_bindings.json",
        ))
        .unwrap();
        let builder = Compiler::builder();
        let c = builder.create(&m, &b).expect("compile duplicate_imports");
        let mdata = c.module_data().unwrap();

        assert_eq!(mdata.import_functions().len(), 2);
        assert_eq!(mdata.import_functions()[0].module, "env");
        assert_eq!(mdata.import_functions()[0].name, "read");
        assert_eq!(mdata.import_functions()[1].module, "env");
        assert_eq!(mdata.import_functions()[1].name, "write");
        assert_eq!(mdata.function_info().len(), 5);
        assert_eq!(mdata.function_info()[0].name, Some("host_read"));
        assert_eq!(mdata.function_info()[1].name, Some("host_write"));
        assert_eq!(mdata.function_info()[2].name, Some("guest_func__start"));
        assert_eq!(mdata.export_functions().len(), 3);
        assert_eq!(mdata.export_functions()[0].names, ["read_2", "read"]);
        assert_eq!(mdata.export_functions()[2].names, ["_start"]);
        assert_eq!(mdata.globals_spec().len(), 0);
    }

    #[test]
    fn icall_import() {
        let m = load_wat_module("icall_import");
        let b = Bindings::from_file(&PathBuf::from(
            "tests/bindings/icall_import_test_bindings.json",
        ))
        .unwrap();
        let builder = Compiler::builder();
        let c = builder.create(&m, &b).expect("compile icall");
        let mdata = c.module_data().unwrap();

        assert_eq!(mdata.import_functions().len(), 1);
        assert_eq!(mdata.import_functions()[0].module, "env");
        assert_eq!(mdata.import_functions()[0].name, "icalltarget");
        assert_eq!(mdata.function_info().len(), 7);
        assert_eq!(mdata.export_functions()[0].names, vec!["launchpad"]);
        assert_eq!(mdata.globals_spec().len(), 0);

        /*  TODO can't express these with module data
        assert_eq!(
            p.get_table(0).unwrap().elements().get(0),
            Some(&TableElem::FunctionIx(2))
        ); // righttype1
        assert_eq!(
            p.get_table(0).unwrap().elements().get(1),
            Some(&TableElem::FunctionIx(3))
        ); // righttype2
        assert_eq!(
            p.get_table(0).unwrap().elements().get(2),
            Some(&TableElem::FunctionIx(4))
        ); // wrongtype
        assert_eq!(
            p.get_table(0).unwrap().elements().get(3),
            Some(&TableElem::FunctionIx(0))
        ); // righttype_imported
        assert_eq!(p.get_table(0).unwrap().elements().get(4), None);
        */
    }

    #[test]
    fn icall() {
        let m = load_wat_module("icall");
        let b = Bindings::empty();
        let builder = Compiler::builder();
        let c = builder.create(&m, &b).expect("compile icall");
        let _module_data = c.module_data().unwrap();

        /*  TODO can't express these with module data
        assert_eq!(
            p.get_table(0).unwrap().elements().get(0),
            Some(&TableElem::FunctionIx(1))
        ); // righttype1
        assert_eq!(
            p.get_table(0).unwrap().elements().get(1),
            Some(&TableElem::FunctionIx(2))
        ); // righttype2
        assert_eq!(
            p.get_table(0).unwrap().elements().get(2),
            Some(&TableElem::FunctionIx(3))
        ); // wrongtype
        assert_eq!(p.get_table(0).unwrap().elements().get(4), None);
        */
    }

    #[test]
    fn icall_sparse() {
        let m = load_wat_module("icall_sparse");
        let b = Bindings::empty();
        let builder = Compiler::builder();
        let c = builder.create(&m, &b).expect("compile icall_sparse");
        let _module_data = c.module_data().unwrap();

        /*  TODO can't express these with module data
        assert_eq!(
            p.get_table(0).unwrap().elements().get(0),
            Some(&TableElem::Empty)
        );
        assert_eq!(
            p.get_table(0).unwrap().elements().get(1),
            Some(&TableElem::FunctionIx(1))
        ); // righttype1
        assert_eq!(
            p.get_table(0).unwrap().elements().get(2),
            Some(&TableElem::FunctionIx(2))
        ); // righttype2
        assert_eq!(
            p.get_table(0).unwrap().elements().get(3),
            Some(&TableElem::FunctionIx(3))
        ); // wrongtype
        assert_eq!(
            p.get_table(0).unwrap().elements().get(4),
            Some(&TableElem::Empty)
        );
        assert_eq!(
            p.get_table(0).unwrap().elements().get(5),
            Some(&TableElem::Empty)
        );
        assert_eq!(p.get_table(0).unwrap().elements().get(6), None);
        */
    }

    #[test]
    fn globals_import() {
        use lucet_module::Global as GlobalVariant;
        let m = load_wat_module("globals_import");
        let b = Bindings::empty();
        let builder = Compiler::builder();
        let c = builder.create(&m, &b).expect("compile globals_import");
        let module_data = c.module_data().unwrap();
        let gspec = module_data.globals_spec();

        assert_eq!(gspec.len(), 1);
        let g = gspec.get(0).unwrap().global();
        match g {
            GlobalVariant::Import { module, field } => {
                assert_eq!(*module, "env");
                assert_eq!(*field, "x");
            }
            _ => panic!("global should be an import"),
        }
    }

    #[test]
    fn heap_spec_import() {
        use lucet_module::HeapSpec;
        let m = load_wat_module("heap_spec_import");
        let b = Bindings::empty();
        let h = HeapSettings::default();
        let builder = Compiler::builder().with_heap_settings(h.clone());
        let c = builder.create(&m, &b).expect("compiling heap_spec_import");

        assert_eq!(
            c.module_data().unwrap().heap_spec(),
            Some(&HeapSpec {
                // reserved and guard are given by HeapSettings
                reserved_size: h.min_reserved_size,
                guard_size: h.guard_size,
                // initial size of import specified as 6 wasm pages
                initial_size: 6 * 64 * 1024,
                // max size of import is specified as 10 wasm pages
                max_size: Some(10 * 64 * 1024),
            })
        );
    }

    #[test]
    fn heap_spec_definition() {
        use lucet_module::HeapSpec;
        let m = load_wat_module("heap_spec_definition");
        let b = Bindings::empty();
        let h = HeapSettings::default();
        let builder = Compiler::builder().with_heap_settings(h.clone());
        let c = builder
            .create(&m, &b)
            .expect("compiling heap_spec_definition");

        assert_eq!(
            c.module_data().unwrap().heap_spec(),
            Some(&HeapSpec {
                // reserved and guard are given by HeapSettings
                reserved_size: h.min_reserved_size,
                guard_size: h.guard_size,
                // initial size defined as 5 wasm pages
                initial_size: 5 * 64 * 1024,
                // no max size defined
                max_size: None,
            })
        );
    }

    #[test]
    fn heap_spec_none() {
        let m = load_wat_module("heap_spec_none");
        let b = Bindings::empty();
        let builder = Compiler::builder();
        let c = builder.create(&m, &b).expect("compiling heap_spec_none");
        assert_eq!(c.module_data().unwrap().heap_spec(), None,);
    }

    #[test]
    fn oversize_data_segment() {
        use lucetc::Error as LucetcError;
        let m = load_wat_module("oversize_data_segment");
        let b = Bindings::empty();
        let builder = Compiler::builder();
        let c = builder.create(&m, &b);
        assert!(
            c.is_err(),
            "compilation error because data initializers are oversized"
        );
        assert!(if let LucetcError::InitData = c.err().unwrap() {
            true
        } else {
            false
        });
    }

    #[test]
    fn element_out_of_range() {
        use lucetc::Error as LucetcError;
        let m = load_wat_module("element_out_of_range");
        let b = Bindings::empty();
        let builder = Compiler::builder();
        let c = builder.create(&m, &b).unwrap();
        match c.object_file() {
            Err(LucetcError::ElementInitializerOutOfRange(_, _)) => (),
            Ok(_) | Err(_) => panic!("unexpected result"),
        }
    }

    // XXX adding more negative tests like the one above is valuable - lets do it

    #[test]
    fn invalid_module() {
        use lucetc::Error as LucetcError;
        use std::fs::File;
        use std::io::Read;
        // I used the `wast2json` tool to produce the file invalid.wasm from an assert_invalid part
        // of a spectest (call.wast)
        let wasmfile = PathBuf::from("tests/wasm/invalid.wasm");
        let mut m = Vec::new();
        let mut file = File::open(&wasmfile).expect("open module file");
        file.read_to_end(&mut m).expect("read contents of module");

        let b = Bindings::empty();
        let builder = Compiler::builder();
        let c = builder.create(&m, &b);
        assert!(
            c.is_err(),
            "compilation error because wasm module is invalid"
        );
        assert!(if let LucetcError::WasmValidation(_) = c.err().unwrap() {
            true
        } else {
            false
        });
    }

    #[test]
    fn start_section() {
        let m = load_wat_module("start_section");
        let b = Bindings::empty();
        let builder = Compiler::builder();
        let _c = builder.create(&m, &b).expect("compile start_section");
        /*
        assert!(
            p.module().start_section().is_some(),
            "start section is defined"
        );
        */
    }

    #[test]
    fn names_local() {
        let m = load_wat_module("names_local");
        let b = super::test_bindings();
        let builder = Compiler::builder();
        let c = builder.create(&m, &b).expect("compile names_local");
        let mdata = c.module_data().unwrap();

        assert_eq!(mdata.import_functions().len(), 0);
        assert_eq!(mdata.export_functions().len(), 0);
        assert_eq!(mdata.function_info().len(), 3);
        assert_eq!(
            mdata.function_info().get(0).unwrap().name,
            Some("func_name_0")
        )
    }
}

mod compile {
    // Tests for compilation completion
    use super::load_wat_module;
    use lucetc::Compiler;
    fn run_compile_test(file: &str) {
        let m = load_wat_module(file);
        let b = super::test_bindings();
        let builder = Compiler::builder();
        let c = builder
            .create(&m, &b)
            .unwrap_or_else(|_| panic!("compile {}", file));
        let _obj = c
            .object_file()
            .unwrap_or_else(|_| panic!("codegen {}", file));
    }
    macro_rules! compile_test {
        ($base_name:ident) => {
            #[test]
            fn $base_name() {
                run_compile_test(stringify!($base_name))
            }
        };
    }

    compile_test!(arith);
    compile_test!(call);
    compile_test!(data_segment);
    compile_test!(fibonacci);
    compile_test!(globals_definition);
    compile_test!(globals_import);
    compile_test!(icall);
    compile_test!(icall_import);
    compile_test!(icall_sparse);
    compile_test!(import);
    compile_test!(import_many);
    compile_test!(locals);
    compile_test!(locals_csr);
    compile_test!(memory);
    compile_test!(return_at_end);
    compile_test!(current_memory);
    compile_test!(grow_memory);
    compile_test!(unreachable_code);
    compile_test!(start_section);
}

mod validate {
    use super::load_wat_module;
    use lucet_validate::Validator;
    use lucetc::{Compiler, CpuFeatures, HeapSettings, OptLevel};
    use target_lexicon::Triple;

    #[test]
    fn validate_arith() {
        let m = load_wat_module("arith");
        let b = super::test_bindings();

        // Empty witx: arith module has no imports
        let v = Validator::parse("")
            .expect("empty witx validates")
            .with_wasi_exe(false);
        let builder = Compiler::builder().with_validator(Some(v));
        let c = builder.create(&m, &b).expect("compile");
        let _obj = c.object_file().expect("codegen");
    }

    #[test]
    fn validate_import() {
        let m = load_wat_module("import");
        let b = super::test_bindings();

        let witx = "
            (module $env
              (@interface func (export \"inc\")
                (result $r s32)))";
        let v = Validator::parse(witx)
            .expect("witx validates")
            .with_wasi_exe(false);

        let builder = Compiler::builder().with_validator(Some(v));
        let c = builder.create(&m, &b).expect("compile");
        let _obj = c.object_file().expect("codegen");
    }

    #[test]
    fn validate_icall_import() {
        let m = load_wat_module("icall_import");
        let b = super::test_bindings();

        let witx = "
            (module $env
              (@interface func (export \"icalltarget\")
                (param $a1 u32)
                (param $a2 u32)
                (result $r s32)))";
        let v = Validator::parse(witx)
            .expect("witx validates")
            .with_wasi_exe(false);

        let builder = Compiler::builder().with_validator(Some(v));
        let c = builder.create(&m, &b).expect("compile");
        let _obj = c.object_file().expect("codegen");
    }

    #[test]
    fn validate_exported_import() {
        let m = load_wat_module("exported_import");
        let b = super::test_bindings();

        let witx = "
            (module $env
              (@interface func (export \"imported_main\"))
              (@interface func (export \"inc\")))";
        let v = Validator::parse(witx)
            .expect("witx validates")
            .with_wasi_exe(false);

        let builder = Compiler::builder().with_validator(Some(v));
        let c = builder.create(&m, &b).expect("compile");
        let _obj = c.object_file().expect("codegen");
    }

    #[test]
    fn validate_multiple_import() {
        let m = load_wat_module("multiple_import");
        let b = super::test_bindings();
        let h = HeapSettings::default();

        let witx = "
            (module $env
              (@interface func (export \"imported_main\"))
              (@interface func (export \"inc\")))";
        let v = Validator::parse(witx)
            .expect("witx validates")
            .with_wasi_exe(false);

        let c = Compiler::new(
            &m,
            Triple::host(),
            OptLevel::default(),
            CpuFeatures::default(),
            &b,
            h,
            false,
            &Some(v),
            false,
        )
        .expect("compile");
        let _obj = c.object_file().expect("codegen");
    }

    #[test]
    fn validate_import_many() {
        let m = load_wat_module("import_many");
        let b = super::test_bindings();

        let witx = "
            (module $env
              (@interface func (export \"imp_0\") (result $r u32))
              (@interface func (export \"imp_1\") (result $r u32))
              (@interface func (export \"imp_2\") (result $r u32))
              (@interface func (export \"imp_3\") (result $r u32)))";
        let v = Validator::parse(witx)
            .expect("witx validates")
            .with_wasi_exe(false);

        let builder = Compiler::builder().with_validator(Some(v));
        let c = builder.create(&m, &b).expect("compile");
        let _obj = c.object_file().expect("codegen");
    }

    #[test]
    fn validate_wasi_exe() {
        let m = load_wat_module("wasi_exe");
        let b = super::test_bindings();

        let witx = "";
        let v = Validator::parse(witx)
            .expect("witx validates")
            .with_wasi_exe(true);

        let builder = Compiler::builder().with_validator(Some(v));
        let c = builder.create(&m, &b).expect("compile");
        let _obj = c.object_file().expect("codegen");
    }
}
