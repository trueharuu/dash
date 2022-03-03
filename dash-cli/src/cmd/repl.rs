use clap::ArgMatches;
use dash::vm::Vm;
use rustyline::Editor;

pub fn repl() -> anyhow::Result<()> {
    let mut rl = Editor::<()>::new();

    while let Ok(input) = rl.readline("> ") {
        if input.is_empty() {
            break;
        }

        rl.add_history_entry(&input);

        match dash::eval(&input) {
            Ok((_vm, value)) => {
                println!("{:?}", value);
            }
            Err(err) => println!("{}", err),
        }
    }

    Ok(())
}
