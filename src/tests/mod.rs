use crate::{
    diff::{build_diff, DiffType},
    snapshot::make_snapshot,
};
use std::path::Path;

#[test]
fn diff() {
    println!("\n=============\n");

    let mut args = std::env::args().skip(3);

    println!("Building source snapshot...");
    let source = make_snapshot(Path::new(
        &args.next().expect("Please provide a source directory"),
    ))
    .unwrap();

    println!("Building destination snapshot...");
    let dest = make_snapshot(Path::new(
        &args.next().expect("Please provide a destination directory"),
    ))
    .unwrap();

    println!("Diffing...");
    let diff = build_diff(source, dest);

    println!("Done!");
    println!("\n=============\n");

    for item in diff {
        let sym = match item.status {
            DiffType::Added => "+",
            DiffType::Changed => "~",
            DiffType::TypeChanged => "!",
            DiffType::Deleted => "-",
        };

        println!("{} {}", sym, item.path.display());
    }
}
