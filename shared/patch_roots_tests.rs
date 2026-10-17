use super::*;

fn key(bundle: &str) -> PluginKey {
    PluginKey {
        format: "CLAP".into(),
        id: "test.sampler".into(),
        name: "Sampler".into(),
        vendor: "Test".into(),
        bundle_path: bundle.into(),
    }
}

fn fixture(roots: &[&str], files: &[&str]) -> Fixture {
    Fixture {
        table: Table {
            plugins: vec![Plugin {
                format: "CLAP".into(),
                id: "test.sampler".into(),
                bundle_path: r"C:\Plugins\Sampler.clap".into(),
                roots: roots.iter().map(|r| r.to_string()).collect(),
            }],
        },
        files: files.iter().map(|f| f.to_string()).collect(),
        refreshed: 0,
    }
}

fn catalog(bundle: &str, path: &str, file: &str) -> PatchReference {
    PatchReference {
        plugin: key(bundle),
        source: Source::Catalog {
            path: path.into(),
            file: file.into(),
        },
        display: "Organ".into(),
    }
}

#[test]
fn relative_paths_use_slashes_and_ignore_verbatim_prefix_and_case() {
    assert_eq!(
        relative(Path::new(r"\\?\D:\Lib\Sfz\Organ\Pedal.sfz"), r"d:\lib\sfz").as_deref(),
        Some("Organ/Pedal.sfz")
    );
    assert_eq!(
        relative(Path::new(r"D:\Lib\Sfz/Organ"), r"D:\Lib\Sfz\").as_deref(),
        Some("Organ")
    );
    assert_eq!(
        relative(Path::new(r"D:\Lib"), r"D:\Lib").as_deref(),
        Some("")
    );
    assert_eq!(relative(Path::new(r"D:\Other\a.sfz"), r"D:\Lib"), None);
    assert_eq!(relative(Path::new(r"D:\Lib\..\a.sfz"), r"D:\Lib"), None);
    assert_eq!(
        join(r"D:\Lib", "Organ/Pedal.sfz"),
        Path::new(r"D:\Lib\Organ\Pedal.sfz")
    );
}

#[test]
fn smf_form_has_no_machine_paths_and_restores_the_same_file() {
    let mut roots = fixture(&[r"D:\Lib\Sfz"], &[r"D:\Lib\Sfz\Organ\Pedal.sfz"]);
    let runtime = catalog(
        r"C:\Plugins\Sampler.clap",
        r"D:\Lib\Sfz\Organ\Pedal.sfz",
        r"D:\Lib\Sfz\Organ\Pedal.sfz",
    );
    let saved = portable(&runtime, &mut roots).unwrap();
    assert_eq!(saved, catalog("", "Organ/Pedal.sfz", "Organ/Pedal.sfz"));
    let json = serde_json::to_string(&saved).unwrap();
    assert!(!json.contains("bundle_path") && !json.contains('\\') && !json.contains("D:"));
    assert_eq!(resolve(&saved, &mut roots).unwrap(), runtime);
    assert_eq!(roots.refreshed, 0);
}

#[test]
fn program_selector_and_built_in_presets_keep_their_root() {
    let mut roots = fixture(
        &[r"D:\Carts", r"C:\Plugins\Sampler.clap"],
        &[r"D:\Carts\Organ.syx", r"C:\Plugins\Sampler.clap"],
    );
    let program = catalog(
        r"C:\Plugins\Sampler.clap",
        r"D:\Carts\Organ.syx\19 PIPE",
        r"D:\Carts\Organ.syx",
    );
    let saved = portable(&program, &mut roots).unwrap();
    assert_eq!(saved, catalog("", "Organ.syx/19 PIPE", "Organ.syx"));
    assert_eq!(resolve(&saved, &mut roots).unwrap(), program);
    let built_in = PatchReference {
        source: Source::Effect {
            path: r"C:\Plugins\Sampler.clap".into(),
            value: "Large Hall".into(),
        },
        ..program
    };
    let saved = portable(&built_in, &mut roots).unwrap();
    assert_eq!(
        saved.source,
        Source::Effect {
            path: "".into(),
            value: "Large Hall".into()
        }
    );
    assert_eq!(resolve(&saved, &mut roots).unwrap(), built_in);
}

#[test]
fn moved_roots_are_rediscovered_once_and_never_guessed() {
    let saved = catalog("", "Organ/Pedal.sfz", "Organ/Pedal.sfz");
    let mut moved = fixture(&[r"D:\Old"], &[r"E:\New\Organ\Pedal.sfz"]);
    assert!(resolve(&saved, &mut moved).is_err());
    assert_eq!(moved.refreshed, 1);
    let mut both = fixture(
        &[r"D:\A", r"D:\B"],
        &[r"D:\A\Organ\Pedal.sfz", r"D:\B\Organ\Pedal.sfz"],
    );
    let error = resolve(&saved, &mut both).unwrap_err();
    assert!(error.contains("several"), "{error}");
    let outside = catalog(r"C:\Plugins\Sampler.clap", r"F:\x.sfz", r"F:\x.sfz");
    assert!(portable(&outside, &mut both).is_err());
}

#[test]
fn smf_paths_must_be_relative_slash_paths() {
    let mut roots = fixture(&[r"D:\Lib"], &[r"D:\Lib\a.sfz"]);
    for bad in [
        r"D:\Lib\a.sfz",
        "/a.sfz",
        r"Organ\a.sfz",
        "../a.sfz",
        "a//b.sfz",
        "",
    ] {
        assert!(
            resolve(&catalog("", bad, bad), &mut roots).is_err(),
            "{bad}"
        );
    }
    assert!(resolve(&catalog("C:/x.clap", "a.sfz", "a.sfz"), &mut roots).is_err());
    assert!(resolve(&catalog("", "a.sfz", "a.sfz"), &mut roots).is_ok());
}
