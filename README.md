<img height="128" alt="necoDL icon" src="https://github.com/user-attachments/assets/65c94234-faca-4b64-9487-e91cf232f543" />

# NecoDL

A CLI Workshop addon manager for Source Engine dedicated servers.  

I built this as a workaround to recent Steam API issues where valid Workshop entries fail to return data, breaking built-in tools.  
It acts as a full replacement for your server's addon manager, you can subscribe to Workshop items or collections by ID and manage them via commands.  

> ⚠️ If you use this for No More Room in Hell, it will **overwrite `workshop_maps.txt` completely**. Back it up before installing.

---

## Installation & Setup

1. Install SteamCMD: [Downloading SteamCMD](https://developer.valvesoftware.com/wiki/SteamCMD#Downloading_SteamCMD)  
2. Download the latest NecoDL executable from [releases](https://github.com/dysphie/neco-dl/releases).  
   * Place it anywhere—it **does not** need to be in your server root.  
3. Configure `config.toml`:

```toml
steam_cmd = "./steamcmd/steamcmd.exe"  # path to steamcmd (.exe or .sh)
output_dir = "./downloads"             # where downloaded files go (typically your server root)
appid = "224260"

# only allow these files to be downloaded
whitelist = [
    "maps/*.nmo",
    "maps/*.nav",
    "maps/*.txt",
    "maps/*.bsp",
    "maps/maphacks/**/*.txt"
]
```

> ⚠️ Tip: `output_dir` is usually your server root, but it can be anywhere.
> You can keep downloads in a separate folder and mount it in `gameinfo.txt` with `game+mod <path>` so the server(s) can access the files.


4. Run NecoDL:

   * Interactively: `./necodl`
   * Directly: `./necodl update`

---

## Commands

| Command         | Description                                                   | Flags & Options                              |
| --------------- | ------------------------------------------------------------- | -------------------------------------------- |
| `download <id>` | Download a Workshop item or collection of items               | `-f, --force`: Redownload even if up-to-date |
| `update`        | Update all subscribed items                                   | `-f, --force`: Redownload even if up-to-date |
| `list`          | Show subscribed items                                         | `-v, --verbose`: Display detailed file info  |
| `remove <id>`   | Unsubscribe + delete files (cleans orphaned collection items) |                                              |
| `info`          | Display config, storage usage, and stats                      |                                              |
| `help`          | Show this command reference                                   |                                              |
| `exit`/`quit`   | Exit                                                          |                                              |

---

## Examples

* Download a single map, [Subside](https://steamcommunity.com/sharedfiles/filedetails/?id=1480550740):

```bash
./necodl download 1480550740
```

* Update all subscribed maps:

```bash
./necodl update
```

* Update maps automatically every hour via cron:

```bash
0 * * * * /path/to/necodl update
```
