use chrono::prelude::Utc;
use chrono::NaiveDateTime;
use colored::*;
use exitfailure::ExitFailure;
use log::{info, warn};
use regex::{Regex, RegexSet, SubCaptureMatches};
use rusqlite::types::ToSql;
use std::io::Write;
use structopt::StructOpt;

use std::{
    env::{temp_dir, var},
    fs::File,
    io::Read,
    process::Command,
    process::Stdio,
};

use log;

use rusqlite::{params, Connection, Result};

#[derive(Debug)]
struct Note {
    id: i32,
    created_at: NaiveDateTime,
    last_edited: NaiveDateTime,
    text: String,
}

#[derive(Debug)]
struct Tag {
    id: i32,
    created_at: NaiveDateTime,
    last_edited: NaiveDateTime,
    name: String,
}

struct TagWithNote {
    tag: Tag,
    note: Note,
}

/// Search for a pattern in a file and display the lines that contain it.
#[derive(StructOpt)]
enum Notes {
    Add { tags: String },
    Attach { tags: String, note: i32 },
    Show { toShow: Option<i32> },
    Find { toFind: String },
    Edit { toEdit: String },
    Rm { toDelete: i32 },
}

fn get_db_conn() -> Connection {
    let conn = Connection::open("db.sql").unwrap();
    conn.execute_batch("PRAGMA foreign_keys=1").unwrap();
    conn
}

fn get_create_tag(tag_name: &String) -> Tag {
    let conn = get_db_conn();
    let mut stmt = conn.prepare("SELECT * FROM tags where name=?").unwrap();
    let tag_iter = stmt
        .query_map(params![tag_name], |row| {
            Ok(Tag {
                id: row.get(0)?,
                created_at: row.get(1)?,
                last_edited: row.get(2)?,
                name: row.get(3)?,
            })
        })
        .unwrap();

    for tag in tag_iter {
        return tag.unwrap();
    }

    // doesn't exist, create tag
    conn.execute(
        "INSERT INTO tags (created_at, last_edited, name)
                    VALUES (datetime('now'), datetime('now'), ?)",
        &[tag_name],
    );
    Tag {
        name: String::from(tag_name),
        id: conn.last_insert_rowid() as i32,
        last_edited: Utc::now().naive_utc(),
        created_at: Utc::now().naive_utc(),
    }
}

fn add(tags: String) {
    let editor = "vim";
    let mut file_path = temp_dir();
    file_path.push("editable");
    File::create(&file_path).expect("Could not create file");

    Command::new(editor)
        .arg(&file_path)
        .status()
        .expect("Something went wrong");
    let mut editable = String::new();
    File::open(file_path)
        .expect("Could not open file")
        .read_to_string(&mut editable);

    let conn = get_db_conn();
    conn.execute(
        "INSERT INTO notes (created_at, last_edited, text)
                  VALUES (datetime('now'), datetime('now'), ?1)",
        params![editable],
    );
    let note_id = conn.last_insert_rowid();
    let tags_iter = tags.split(',');

    for t in tags_iter {
        let tag = get_create_tag(&t.to_string());
        conn.execute(
            "INSERT INTO tag_on_note (tag_id, note_id)
                    VALUES (?1, ?2)",
            params![tag.id, note_id],
        );
    }
}

fn print_hightlighted_text(text: &String, reg: &Regex) {
    let splitted = reg.split(&text);
    let res = reg.captures(&text);
    match res {
        None => print!("{}", &text),
        Some(caps) => {
            let mut idx = 0;
            for split in splitted {
                print!(
                    "{}{}",
                    split,
                    &caps.get(idx).map_or("", |m| m.as_str()).red()
                );
                idx = idx + 1;
            }
        }
    }
}

fn find(str_to_find: String) {
    let conn = get_db_conn();
    let mut stmt = conn
        .prepare("SELECT id, created_at, last_edited, text FROM notes")
        .unwrap();
    let note_iter = stmt
        .query_map(params![], |row| {
            Ok(Note {
                id: row.get(0)?,
                created_at: row.get(1)?,
                last_edited: row.get(2)?,
                text: row.get(3)?,
            })
        })
        .unwrap();

    for note in note_iter {
        //println!("{:?}", note);
        let note = note.unwrap();
        let re = Regex::new(&str_to_find).unwrap();
        let note_text = note.text.clone();
        let matched = re.is_match(&note_text);

        if matched {
            let mut shortened_note = note_text.clone();
            shortened_note.truncate(30);
            print!("• ({}) -- ", &note.id.to_string().yellow());
            print_hightlighted_text(&shortened_note.replace('\n', " "), &re);
            println!("");
        }
        //if note_text
        //    .to_lowercase()
        //    .contains(&str_to_find.to_lowercase())
        //{
        //    println!("Found note {:?}", note_text);
        //}
    }
}

fn delete_note(toDelete: i32) {
    let conn = get_db_conn();
    conn.execute("DELETE FROM notes WHERE id=?", params![toDelete]);
}

fn main() -> Result<(), ExitFailure> {
    env_logger::init();
    info!("Starting");
    let args = Notes::from_args();
    match args {
        Notes::Add { tags } => {
            println!("adding: {}", tags);
            add(tags)
        }
        Notes::Attach { tags, note } => println!("attaching: {}", note),
        Notes::Show { toShow } => {
 
            match toShow {
                None => {println!("show tags")},
                Some(val) => {println!("show individual note")},
            }
        }
        Notes::Find { toFind } => {
            println!("Searching for: {}", toFind.red());
            find(toFind);
        }
        Notes::Edit { toEdit } => println!("adding: {}", toEdit),
        Notes::Rm { toDelete } => delete_note(toDelete),
    }

    Ok(())
}
