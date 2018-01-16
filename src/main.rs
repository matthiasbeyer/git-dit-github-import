extern crate clap;
extern crate hubcaps;
extern crate hyper;
extern crate futures;
extern crate tokio_core;
extern crate indicatif;
extern crate libgitdit;
extern crate git2;
extern crate chrono;

use tokio_core::reactor::Core;
use hubcaps::Github;
use hubcaps::comments::{Comment, CommentListOptions};
use hubcaps::issues::{Issue, Issues, IssueListOptions, IssueRef, State, Sort};
use clap::{App, Arg};
use git2::Repository;
use git2::Signature;
use libgitdit::RepositoryExt;
use libgitdit::Message;
use chrono::DateTime;

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
        .arg(Arg::with_name("dryrun")
             .long("dry-run")
             .help("Set to not actually import anything into git-dit"))
        .get_matches();

    let progress = indicatif::ProgressBar::new_spinner();
    progress.enable_steady_tick(250);
    progress.set_message("Parsing commandline");

    let dry_run    = matches.is_present("dryrun");
    let username   = matches.value_of("username").expect("No username");
    let reponame   = matches.value_of("reponame").expect("No reponame");
    let useragent  = String::from("user-agent-name");
    let mut core   = Core::new().expect("Cannot initialize tokei Core");
    let github     = Github::new(useragent, None, &core.handle());
    let issues     = Issues::new(github, username, reponame);

    let issue_list_options = IssueListOptions::builder()
        .state(State::All)
        .sort(Sort::Created)
        .build();

    progress.set_message("Fetching issue list");
    let issue_list = core.run(issues.list(&issue_list_options)).expect("Failed to fetch issues!");
    let comment_list_options = CommentListOptions::builder().build();

    if dry_run {
        for issue in issue_list.iter() {
            progress.set_message(&format!("Fetching issue {}", issue.number));
            let iref : IssueRef<_> = Issues::get(&issues, issue.number);
            print_issue_information(&issue);

            core.run(iref.comments()
                .list(&comment_list_options))
                .expect(&format!("Failed to get comments for issue {}", issue.number))
                .into_iter()
                .for_each(|c| print_comment_information(&c))
        }
    } else {
        let repo      = Repository::open_from_env().expect("Failed to open repository");
        let committer = repo.signature().expect("Failed to get committer signature");

        for issue in issue_list.iter() {
            progress.set_message(&format!("Fetching issue {}", issue.number));
            let iref : IssueRef<_> = Issues::get(&issues, issue.number);

            let dit_issue = {
                let author  = signature_for(&issue.user.login, &issue.created_at);
                let message = &issue.body;
                let tree    = repo.empty_tree().expect("Failed to create empty tree");
                let parents = vec![];
                repo.create_issue(&author, &committer, &message, &tree, &parents)
                    .expect("Failed creating issue")
            };

            let dit_issue_commit = dit_issue.initial_message().expect("Failed to get initial message");
            let mut parent       = dit_issue_commit;
            let comments         = core.run(iref.comments()
                .list(&comment_list_options))
                .expect(&format!("Failed to get comments for issue {}", issue.number))
                .into_iter();

            for comment in comments {
                let author  = signature_for(&comment.user.login, &comment.created_at);
                let subject = parent.reply_subject();
                let tree    = parent.tree().expect("Failed to get tree from parent");
                let message = if let Some(subj) = subject {
                    format!("{subject}\n\n{body}", subject = subj, body = &comment.body)
                } else {
                    comment.body.clone()
                };

                let new_parent = {
                    let parent_refs = Some(&parent).into_iter();
                    dit_issue.add_message(&author, &committer, message, &tree, parent_refs)
                        .expect("Failed to add message")
                };

                parent = new_parent;
            }
         }
    }

    progress.finish_with_message("Done");
}

fn print_issue_information(i: &Issue) {
    println!(
r#"# {number} - {author} - {id}

  ## {title}

  {body}

  ---------------------
  created: {created_at}
  updated: {updated_at}
  labels: {labels:?}
  {comments} comments
  ---------------------"#,
          number     = i.number     ,
          author     = i.user.login ,
          id         = i.id         ,
          title      = i.title      ,
          body       = i.body       ,
          created_at = i.created_at ,
          updated_at = i.updated_at ,
          labels     = i.labels.iter().map(|l| l.name.clone()).collect::<Vec<String>>(),
          comments   = i.comments   );
}

fn print_comment_information(c: &Comment) {
    println!(
r#"\t# {user} on {created_at}:
\t {body}

"#,
        user = c.user.login,
        created_at = c.created_at,
        body = c.body);
}

fn signature_for(username: &String, created_at: &String) -> Signature<'static> {
    let date = DateTime::parse_from_rfc3339(created_at)
            .expect("Malformed Date. Cannot parse");
    let time = ::git2::Time::new(date.timestamp(), date.offset().local_minus_utc() / 60);
    Signature::new(username, "unknown@email.tld", &time)
        .expect("Failed to construct a Signature")
}

