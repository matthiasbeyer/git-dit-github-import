extern crate clap;
extern crate hubcaps;
extern crate hyper;
extern crate futures;
extern crate tokio_core;
extern crate indicatif;

use tokio_core::reactor::Core;
use hubcaps::Github;
use futures::future::Future;
use hubcaps::comments::CommentListOptions;
use hubcaps::issues::{Issues, IssueListOptions, IssueRef};
use clap::{App, Arg};

fn main() {
    let matches = App::new("git-dit github-import")
        .version("0.1")
        .author("Matthias Beyer <mail@beyermatthias.de>")
        .about("Import issues from github into git-dit")
        .arg(Arg::with_name("username")
             .index(1)
             .value_name("USER")
             .help("Set the user from which to import the issues"))
        .arg(Arg::with_name("reponame")
             .index(2)
             .value_name("REPO")
             .help("Set the repo from which to import the issues"))
        .get_matches();

    let progress = indicatif::ProgressBar::new_spinner();
    progress.enable_steady_tick(250);
    progress.set_message("Parsing commandline");

    let username   = matches.value_of("username").expect("No username");
    let reponame   = matches.value_of("reponame").expect("No reponame");
    let useragent  = String::from("user-agent-name");
    let mut core   = Core::new().expect("Cannot initialize tokei Core");
    let github     = Github::new(useragent, None, &core.handle());
    let issues     = Issues::new(github, username, reponame);

    progress.set_message("Fetching issue list");
    let issue_list = core.run(issues.list(&IssueListOptions::default()))
        .expect("Failed to fetch issues!");

    for issue in issue_list.iter() {
        progress.set_message(&format!("Fetching issue {}", issue.number));
        let iref : IssueRef<_> = Issues::get(&issues, issue.number);
        core.run(iref.comments()
            .list(&CommentListOptions::default()))
            .expect(&format!("Failed to get comments for issue {}", issue.number))
            .into_iter()
            .for_each(|comment| {
                println!("Got comment {} for issue {} from {}", comment.id, issue.number, issue.user.login);
            });
     }

    progress.finish_with_message("Done");
}

