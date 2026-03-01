use clap::Parser;
use tomt::Args;

fn main() {
    let args = Args::parse();
    let mut count = 0;
    for event in tomt::run(&args).expect("failed to initialize formatter") {
        match event {
            tomt::FormatEvent::Done { success } => {
                if success {
                    if args.check {
                        println!("{count} files checked");
                    } else {
                        if count == 0 {
                            println!("no files formatted");
                        } else {
                            println!("formatted {count} files");
                        }
                    }
                } else {
                    if args.check {
                        println!("some files would change if formatted");
                    } else {
                        println!("failed to format files");
                    }
                    std::process::exit(1);
                }
            }
            tomt::FormatEvent::File(fp) => {
                println!("tomt: {}", fp.display());
                count += 1;
            }
            tomt::FormatEvent::FileError(fp, err) => {
                println!("ERROR: {err}: {}", fp.display());
            }
        }
    }
}
