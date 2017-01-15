extern crate clap;
extern crate env_logger;
extern crate guzuta;

fn main() {
    env_logger::init().unwrap();

    let app = clap::App::new("guzuta")
        .version("0.0.0")
        .about("Custom repository manager for ArchLinux pacman")
        .setting(clap::AppSettings::SubcommandRequired)
        .subcommand(clap::SubCommand::with_name("build")
            .about("Build package in systemd-nspawn environment")
            .arg(clap::Arg::with_name("chroot-dir")
                .long("chroot-dir")
                .takes_value(true)
                .required(true)
                .help("Path to chroot top"))
            .arg(clap::Arg::with_name("package-key")
                .long("package-key")
                .takes_value(true)
                .help("GPG key to sign packages"))
            .arg(clap::Arg::with_name("srcdest")
                .long("srcdest")
                .takes_value(true)
                .help("Path to the directory to store sources"))
            .arg(clap::Arg::with_name("logdest")
                .long("logdest")
                .takes_value(true)
                .help("Path to the directory to store logs"))
            .arg(clap::Arg::with_name("repo-dir")
                .long("repo-dir")
                .takes_value(true)
                .required(true)
                .help("Path to the repository directory"))
            .arg(clap::Arg::with_name("repo-key")
                .long("repo-key")
                .takes_value(true)
                .help("GPG key to sign repository database"))
            .arg(clap::Arg::with_name("arch")
                .long("arch")
                .takes_value(true)
                .required(true)
                .help("Architecture"))
            .arg(clap::Arg::with_name("repo-name")
                .long("repo-name")
                .takes_value(true)
                .required(true)
                .help("Repository name"))
            .arg(clap::Arg::with_name("package-dir")
                .required(true)
                .help("Path to the directory containing PKGBUILD")))
        .subcommand(clap::SubCommand::with_name("repo-add")
            .about("Add PACKAGE_PATH to DB_PATH")
            .arg(clap::Arg::with_name("repo-key")
                .long("repo-key")
                .takes_value(true)
                .help("GPG key to sign repository database"))
            .arg(clap::Arg::with_name("PACKAGE_PATH")
                .required(true)
                .help("Path to package to be added"))
            .arg(clap::Arg::with_name("DB_PATH")
                .required(true)
                .help("Path to repository database")))
        .subcommand(clap::SubCommand::with_name("repo-remove")
            .about("Remove PACKAGE_NAME from DB_PATH")
            .arg(clap::Arg::with_name("repo-key")
                .long("repo-key")
                .takes_value(true)
                .help("GPG key to sign repository database"))
            .arg(clap::Arg::with_name("PACKAGE_NAME")
                .required(true)
                .help("Path to package to be removed"))
            .arg(clap::Arg::with_name("DB_PATH")
                .required(true)
                .help("Path to repository database")))
        .subcommand(clap::SubCommand::with_name("files-add")
            .about("Add PACKAGE_PATH to FILES_PATH")
            .arg(clap::Arg::with_name("repo-key")
                .long("repo-key")
                .takes_value(true)
                .help("GPG key to sign repository database"))
            .arg(clap::Arg::with_name("PACKAGE_PATH")
                .required(true)
                .help("Path to package to be added"))
            .arg(clap::Arg::with_name("FILES_PATH")
                .required(true)
                .help("Path to repository database")))
        .subcommand(clap::SubCommand::with_name("files-remove")
            .about("Remove PACKAGE_NAME from FILES_PATH")
            .arg(clap::Arg::with_name("repo-key")
                .long("repo-key")
                .takes_value(true)
                .help("GPG key to sign repository database"))
            .arg(clap::Arg::with_name("PACKAGE_NAME")
                .required(true)
                .help("Path to package to be removed"))
            .arg(clap::Arg::with_name("DB_PATH")
                .required(true)
                .help("Path to repository database")));
    let matches = app.get_matches();

    run_subcommand(matches.subcommand());
}

fn run_subcommand(subcommand: (&str, Option<&clap::ArgMatches>)) {
    match subcommand {
        ("build", Some(build_command)) => build(build_command),
        ("repo-add", Some(repo_add_command)) => {
            repo_add(repo_add_command);
        }
        ("repo-remove", Some(repo_remove_command)) => {
            repo_remove(repo_remove_command);
        }
        ("files-add", Some(files_add_command)) => {
            files_add(files_add_command);
        }
        ("files-remove", Some(files_remove_command)) => {
            files_remove(files_remove_command);
        }
        _ => {
            panic!("Unknown subcommand");
        }
    }
}

fn build(args: &clap::ArgMatches) {
    let arch = match args.value_of("arch").unwrap() {
        "i686" => guzuta::Arch::I686,
        "x86_64" => guzuta::Arch::X86_64,
        arch => panic!("Unknown architecture: {}", arch),
    };
    let chroot = guzuta::ChrootHelper::new(args.value_of("chroot-dir").unwrap(), arch);
    let package_signer = args.value_of("package-key").map(|key| guzuta::Signer::new(key.to_owned()));
    let builder = guzuta::Builder::new(package_signer,
                                       args.value_of("srcdest").unwrap_or("."),
                                       args.value_of("logdest").unwrap_or("."));
    let repo_dir = std::path::Path::new(args.value_of("repo-dir").unwrap());
    let repo_name = args.value_of("repo-name").unwrap();

    let repo_signer = args.value_of("repo-key").map(|key| guzuta::Signer::new(key.to_owned()));
    let mut db_path = repo_dir.join(repo_name).into_os_string();
    db_path.push(".db");
    let mut files_path = repo_dir.join(repo_name).into_os_string();
    files_path.push(".files");
    let mut db_repo = guzuta::Repository::new(std::path::PathBuf::from(db_path), repo_signer.clone());
    let mut files_repo = guzuta::Repository::new(std::path::PathBuf::from(files_path), repo_signer);
    db_repo.load();
    files_repo.load();

    let package_paths = builder.build_package(args.value_of("package-dir").unwrap(), &repo_dir, &chroot);

    for path in package_paths {
        let package = guzuta::Package::load(&path);
        db_repo.add(&package);
        files_repo.add(&package);
    }

    db_repo.save(false);
    files_repo.save(true);
}

fn repo_add(args: &clap::ArgMatches) {
    let signer = args.value_of("repo-key").map(|key| guzuta::Signer::new(key.to_owned()));
    let package = guzuta::Package::load(&args.value_of("PACKAGE_PATH").unwrap());
    let mut repository = guzuta::Repository::new(std::path::PathBuf::from(args.value_of("DB_PATH").unwrap()),
                                                 signer);

    repository.load();
    repository.add(&package);
    repository.save(false);
}

fn repo_remove(args: &clap::ArgMatches) {
    let signer = args.value_of("repo-key").map(|key| guzuta::Signer::new(key.to_owned()));
    let package_name = args.value_of("PACKAGE_NAME").unwrap();
    let mut repository = guzuta::Repository::new(std::path::PathBuf::from(args.value_of("DB_PATH").unwrap()),
                                                 signer);

    repository.load();
    repository.remove(&package_name);
    repository.save(false);
}

fn files_add(args: &clap::ArgMatches) {
    let signer = args.value_of("repo-key").map(|key| guzuta::Signer::new(key.to_owned()));
    let package = guzuta::Package::load(&args.value_of("PACKAGE_PATH").unwrap());
    let mut repository = guzuta::Repository::new(std::path::PathBuf::from(args.value_of("FILES_PATH").unwrap()),
                                                 signer);

    repository.load();
    repository.add(&package);
    repository.save(true);
}

fn files_remove(args: &clap::ArgMatches) {
    let signer = args.value_of("repo-key").map(|key| guzuta::Signer::new(key.to_owned()));
    let package_name = args.value_of("PACKAGE_NAME").unwrap();
    let mut repository = guzuta::Repository::new(std::path::PathBuf::from(args.value_of("FILES_PATH").unwrap()),
                                                 signer);

    repository.load();
    repository.remove(&package_name);
    repository.save(true);
}
