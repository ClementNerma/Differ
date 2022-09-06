#[macro_export]
macro_rules! fail {
    ($message: tt, $($params: tt)*) => {{
        use colored::Colorize;
        eprintln!("{}", format!($message, $($params)*).bright_red());
        std::process::exit(1);
    }};

    ($message: tt) => {{
        fail!($message,)
    }};
}

#[macro_export]
macro_rules! error {
    ($message: tt, $($params: tt)*) => {{
        use colored::Colorize;
        eprintln!("{}", format!($message, $($params)*).bright_red());
    }};

    ($message: tt) => {{
        error!($message,)
    }};
}

#[macro_export]
macro_rules! warn {
    ($message: tt, $($params: tt)*) => {{
        use colored::Colorize;
        eprintln!("{}", format!($message, $($params)*).bright_yellow());
    }};

    ($message: tt) => {{
        warn!($message,)
    }};
}

#[macro_export]
macro_rules! info {
    ($message: tt, $($params: tt)*) => {{
        use colored::Colorize;
        println!("{}", format!($message, $($params)*).bright_blue());
    }};

    ($message: tt) => {{
        info!($message,)
    }};
}

#[macro_export]
macro_rules! info_inline {
    ($message: tt, $($params: tt)*) => {{
        use colored::Colorize;
        print!("{}", format!($message, $($params)*).bright_blue());
    }};

    ($message: tt) => {{
        info_inline!($message,)
    }};
}

#[macro_export]
macro_rules! success {
    ($message: tt, $($params: tt)*) => {{
        use colored::Colorize;
        println!("{}", format!($message, $($params)*).bright_green());
    }};

    ($message: tt) => {{
        success!($message,)
    }};
}
