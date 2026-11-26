Hi, this is my (so far) simple file tagging utility.

Here are all the commands sorted by their usage.

--Help--

`help` - prints the help menu.

--Database Management--

`database` - prints the location of the database that will be used.
`init-local` - creates and new local database (or does nothing if one already exists),
    `file-tag.sqlite`. It will be used in all subdirectories unless there is a different one below
    to override it.
`backup` - creates a copy of the database.
`stats` - prints database stats, this includes the number of tags and files.

--Tag Management--

`create` - creates a new tag, or makes sure a tag exists, it isn't really necessary as the `tag`
    command already does that.
`remove` - removes a tag or makes sure it doesn't exist.
`rename` - changes the name of a tag.
`merge` - merges 2 tags.
`tags` - lists all the tags in the database (or all the tags on some file).
`associated` - lists all the tags that appear on the same entry as requested tag.

--Aliasing--
`create-alias` - creates a new alias or makes sure one exists.
`remove-alias` - removes an alias or makes sure one doesn't exist.
`aliases` - prints all aliases of a tag or in the database.

--Tagging--

`tag` - adds one or more tags to a file.
`untag` - removes one or more tags from a file, or if no tags are provided removes all the tags.
`dir-tag` - recursively tags everything in a directory.

--Querying--

`query` - finds all the files with or without given tags.
`untagged` - lists all the files from the current directory that are not in the database.
