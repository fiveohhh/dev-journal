use chrono::prelude::Utc;
use chrono::NaiveDateTime;
use colored::*;
use dirs::home_dir;
use exitfailure::ExitFailure;
use log::info;
use regex::Regex;
use structopt::StructOpt;

use std::{env::temp_dir, fs::File, io::Read, process::Command};

use log;

use rusqlite::{params, Connection, Result};

#[derive(Debug)]
struct Note {
    id: i32,
    created_at: NaiveDateTime,
    last_edited: NaiveDateTime,
    text: String,
    tags: Vec<Tag>,
}

#[derive(Debug)]
struct Tag {
    id: i32,
    created_at: NaiveDateTime,
    last_edited: NaiveDateTime,
    name: String,
}

#[derive(Debug)]
struct TagCnt {
    name: String,
    count: i32,
}

#[derive(StructOpt)]
enum Notes {
    /// Add a new note to the specified tag.
    Add {
        /// comma seperated list of tags.  Cannot be numeric
        tags: String,
    },
    /// Not implemented
    Attach { note: i32 },
    /// displays a note or tags in a note
    Show {
        /// If given a tag name, shows notes in tag, if given a note id, displays the note
        to_show: Option<String>,
    },
    /// Find a note
    Find {
        /// input is regex string to match
        to_find: String,
    },
    /// Not Implemented
    Edit { to_edit: String },
    /// deletes the specified note
    Rm {
        /// ID of note to delete
        to_delete: i32,
    },
}

fn get_db_conn() -> Connection {
    let home_dir = home_dir().unwrap().to_str().unwrap().to_owned();
    let conn = Connection::open(home_dir + "/.noteapp/db.sql").unwrap();
    conn.execute_batch("PRAGMA foreign_keys=1").unwrap();

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS notes (
            id INTEGER PRIMARY KEY AUTOINCREMENT, 
            created_at DATETIME NOT NULL, 
            last_edited DATETIME NOT NULL, 
            text VARCHAR NOT NULL
            );
            
            
            CREATE TABLE IF NOT EXISTS tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT, 
            created_at DATETIME NOT NULL, 
            last_edited DATETIME NOT NULL, 
            name  VARCHAR UNIQUE NOT NULL
            );
            
            CREATE TABLE IF NOT EXISTS tag_on_note (
                note_id int,
                tag_id int,
                CONSTRAINT note_tag_pk PRIMARY KEY (note_id, tag_id),
            
                FOREIGN KEY (note_id) REFERENCES notes (id) ON DELETE CASCADE,
            
                FOREIGN KEY (tag_id) REFERENCES tags (id) ON DELETE CASCADE
            
            );",
    )
    .unwrap();
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
    )
    .unwrap();
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
        .read_to_string(&mut editable)
        .unwrap();

    if editable.len() == 0 {
        info!("No note entered.");
        println!("No note entered");
        return;
    }

    let conn = get_db_conn();
    conn.execute(
        "INSERT INTO notes (created_at, last_edited, text)
                  VALUES (datetime('now'), datetime('now'), ?1)",
        params![editable],
    )
    .unwrap();
    let note_id = conn.last_insert_rowid();
    let tags_iter = tags.split(',');

    for t in tags_iter {
        let tag = get_create_tag(&t.to_string());
        conn.execute(
            "INSERT INTO tag_on_note (tag_id, note_id)
                    VALUES (?1, ?2)",
            params![tag.id, note_id],
        )
        .unwrap();
    }
}

fn get_full_note(id: i32) -> Note {
    let conn = get_db_conn();
    let mut note = conn
        .query_row(
            "SELECT id, created_at, last_edited, text FROM notes WHERE id=?",
            &[&id],
            |row| {
                Ok(Note {
                    id: row.get(0).unwrap(),
                    created_at: row.get(1).unwrap(),
                    last_edited: row.get(2).unwrap(),
                    text: row.get(3).unwrap(),
                    tags: vec![],
                })
            },
        )
        .unwrap();
    let mut stmt = conn.prepare("SELECT id, created_at, last_edited, name FROM tags INNER JOIN tag_on_note ON tags.id = tag_on_note.tag_id where tag_on_note.note_id=?").unwrap();
    let tags = stmt
        .query_map(&[&id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                created_at: row.get(1)?,
                last_edited: row.get(2)?,
                name: row.get(3)?,
            })
        })
        .unwrap();

    for tag in tags {
        note.tags.push(tag.unwrap());
    }
    note
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
                tags: vec![],
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

fn delete_note(to_delete: i32) {
    let conn = get_db_conn();
    conn.execute("DELETE FROM notes WHERE id=?", params![to_delete])
        .unwrap();
}

fn display_tags() {
    // get note counte per tag
    let conn = get_db_conn();
    let mut stmt = conn
        .prepare(
            "SELECT tags.name, count(tag_on_note.tag_id) as number_of_notes        
    from tags
    left join tag_on_note
    on (tags.id = tag_on_note.tag_id)
    group by
        tags.id",
        )
        .unwrap();

    let tag_iter = stmt
        .query_map(params![], |row| {
            Ok(TagCnt {
                name: row.get(0)?,
                count: row.get(1)?,
            })
        })
        .unwrap();
    for tag in tag_iter {
        let t = tag.unwrap();
        println!("{} ({})", t.name, t.count.to_string().blue());
    }
}

fn display_notes_with_tag(tag_name: String) {
    // get note counte per tag
    let conn = get_db_conn();
    let mut stmt = conn
        .prepare(
            "SELECT notes.text, notes.id,  notes.created_at, notes.last_edited FROM notes 
INNER JOIN tag_on_note ON notes.id = tag_on_note.note_id
INNER JOIN tags ON tags.id = tag_on_note.tag_id
where tags.name=?;",
        )
        .unwrap();

    let note_iter = stmt
        .query_map(params![tag_name], |row| {
            Ok(Note {
                id: row.get(1).unwrap(),
                text: row.get(0).unwrap(),
                created_at: row.get(2).unwrap(),
                last_edited: row.get(3).unwrap(),
                tags: vec![],
            })
        })
        .unwrap();

    for note in note_iter {
        let note = note.unwrap();
        let mut shortened_note = note.text.clone();
        shortened_note.truncate(30);
        println!(
            "• ({}) -- {}",
            &note.id.to_string().yellow(),
            &shortened_note.replace('\n', " ")
        );
    }
}

fn display_full_note(id: i32) {
    let note = get_full_note(id);
    let mut tags: String = String::from("");
    for tag in note.tags {
        tags.push_str(tag.name.as_str());
        tags.push_str(&", ");
    }

    println!("id:      {}", note.id.to_string().blue());
    println!("created: {}", note.created_at.to_string().blue());
    println!("edited:  {}", note.last_edited.to_string().blue());
    println!("tags:    {}", tags.blue());
    println!("-----------------------content--------------------");
    println!("{}", note.text);
    println!("--------------------------------------------------");
}

// TODO TAG can't have number
fn main() -> Result<(), ExitFailure> {
    env_logger::init();
    info!("Starting");
    let args = Notes::from_args();
    match args {
        Notes::Add { tags } => {
            println!("adding: {}", tags);
            add(tags)
        }
        Notes::Attach { note } => println!("attaching: {}", note),
        Notes::Show { to_show } => match to_show {
            None => display_tags(),
            Some(val) => {
                let val_as_num = val.trim().parse::<i32>();
                match val_as_num {
                    Ok(num) => display_full_note(num),
                    Err(_e) => display_notes_with_tag(val),
                }
            }
        },
        Notes::Find { to_find } => {
            println!("Searching for: {}", to_find.red());
            find(to_find);
        }
        Notes::Edit { to_edit } => println!("NOT IMPLEMENTED: {}", to_edit),
        Notes::Rm { to_delete } => delete_note(to_delete),
    }

    Ok(())
}
