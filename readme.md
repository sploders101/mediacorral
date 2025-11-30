# Mediacorral

This is Mediacorral, the third and hopefully final in my line of media
cataloging tools. This will be a one-stop-shop for managing my media
collection, supporting automated ripping, identification, cataloging, and
disaster recovery.

## Todo

[x] Rewrite backend in Go to avoid Rust's unfortunate async scaling complexities
[ ] Add Ollama-based AI chat to ask questions about rip jobs & subtitles
  * This will assist in scenarios when little information is available for matching
[ ] Use whisper-rs to generate subtitles for analysis when there aren't any
[ ] Add precalculated AI matching recommendations
[ ] Add rudimentary streaming capability (just good enough for identification)
[ ] Add mkvtoolnix-based chapter splitting feature for master tracks
[ ] Add transcoding support to decrease file sizes (without losing disaster recovery info)
[ ] Add ahead-of-time upscaling for high-quality video (since some content can only be purchased in low resolution)


## Preamble

Managing an ethically-obtained media collection is hard. Like, REALLY hard.
It's not just about buying the content. Once you get the content, it often
comes on a ton of discs, and within each disc, the episodes of a TV series, for
example, may not even be in order. Most people just resort to piracy, but I
want to make it easier to do it the right way.

Getting media from a legally-purchased disc onto the system is a painstaking
and tedious process. I've already built a few tools in the past that try to
compare data found on the disc with online catalogues to try and identify which
file is what. However, even with this automation, when you try to go off-grid
like I have for my content and scale up, it's still a massive pain.

Backing up my media would be really expensive. I justified my lack of backups
by the fact that I have all the originals on disc, and I can always restore
from that. Well, that proved to be quite naive, as I failed to take into
account the amount of work it takes to get it off of the disc. So how can I
keep from having to pay for backups of such a large dataset without losing all
the work I put into it? Mediacorral stores hashes of all content, and
information about what they are, as well as backups of some information from
TheMovieDataBase (TMDB) for disaster recovery. By storing hashes of the files,
I can instantly identify what the file is upon ripping it from the disc, and
all I need to back up is those hashes.


## What does it do?

Mediacorral is a ripping, storage, cataloging, and renaming tool all rolled
into one with a fancy web UI.

### Ripping & Cataloging

Upon triggering Mediacorral's ripping process, it will invoke MakeMKV to
extract media from the disc, and save them into a temporary "rips" directory.

Once this is finished the rip will be moved into blob storage and tracked in
a sqlite database. Next, each file will be analyzed, yielding a variety of
information including runtime, resolution, and subtitles.

Next, these files will show up in the "catalogue" section of the web UI, where
you can inform Mediacorral what it might find on the disc. Mediacorral will do
different things based on whether the content is expected to be a TV show or a
Movie.

#### TV Shows

Supported matching strategies:
* OST subtitle similarity scoring
  * This integration downloads subtitles from opensubtitles and scores the
  similarity of each subtitle track ripped from the disc with a subtitle track
  for each possible episode found on opensubtitles.

#### Movies

No movie matching strategies exist yet, as it is usually easy to tell which
track is the main feature from metadata alone. This feature will come, but is
low on the priority list.

### Storage & Renaming

Instead of storing files in user-generated heirarchies, Mediacorral owns the
content and manages it internally in the "blob storage controller". This allows
mediacorral to associate many data points with each file and track them in a
relational database rather than filenames.

#### Exports directories

An exports directory is Mediacorral's way of interfacing with media servers
like Plex, Emby, and Jellyfin. Mediacorral can create link trees using either
symbolic or hard links that utilize standard naming conventions as recognized
by 3rd party media servers. There is no risk to deleting and recreating these
directories, as all data is stored in Mediacorral's "blobs" directory. This
simplifies the process of changing naming schemes and reduces the effect of any
bugs that may exist in Mediacorral's synchronization between database state and
filesystem trees. If an error occurs, just delete it and start again!
