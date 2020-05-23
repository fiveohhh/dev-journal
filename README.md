# Dev Journal

A lightweight application meant to allow developers (or other CLI fans) to
create tagged journal entries or notes that are easily searched/retrievable.

## Usage

### Create a note inline

~~~Bash
devj add mytag,anotherTag -m "Contents of my note"
~~~

### Create a note in default EDITOR

~~~Bash
devj add mytag 
~~~

## Credits

The inspiration for this was derived from [dnote](https://github.com/dnote/dnote).
I loved the idea of dnote, but I wanted to be able to put multiple tags on an note
as well as add attachments (attachments are still a wip).