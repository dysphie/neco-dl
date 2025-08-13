<img height="128" alt="necoDL icon" src="https://github.com/user-attachments/assets/65c94234-faca-4b64-9487-e91cf232f543" />

# NecoDL

A CLI Workshop addon manager for Source Engine dedicated servers.  

I built this as a workaround to recent Steam API issues where valid Workshop entries fail to return data, breaking built-in tools.  
It acts as a full replacement for your server's addon manager, you can subscribe to Workshop items or collections by ID and manage them via commands.  

---

## Installation & Setup

1. Install SteamCMD: [Downloading SteamCMD](https://developer.valvesoftware.com/wiki/SteamCMD#Downloading_SteamCMD)  
2. Download the latest NecoDL executable from [releases](https://github.com/dysphie/neco-dl/releases).
3. Configure `config.toml`:

```toml
steam_cmd = "path/to/steamcmd.sh"       # path to steamcmd (.exe or .sh)
output_dir = "path/to/output/dir"       # directory to place generated files
appid = "224260"                        # game AppID, e.g. 440 (TF2), 730 (CS:GO)

# only allow these files to be downloaded
# never allow everything unless you understand the security risks!
whitelist = [
    "maps/*.nmo",
    "maps/*.nav",
    "maps/*.txt",
    "maps/*.bsp",
    "maps/maphacks/**/*.txt"
]
```

> [!TIP]
> `output_dir` is usually your server root, but you can use a separate folder and mount it in `gameinfo.txt` with `game+mod <path>` so the server(s) can still access the files.

> [!WARNING]
> If you use this for No More Room in Hell, it will overwrite the server's `workshop_maps.txt`. Back it up before installing.

4. Run NecoDL:

   * Interactively: `./necodl`
   * Directly: `./necodl update`

---

## Commands
| Command         | Description                                                   |
| --------------- | ------------------------------------------------------------- |
| `download <id>` | Download a Workshop item or collection of items              <br>`-f`: Redownload even if up-to-date |
| `update`        | Update all subscribed items                                   <br>`-f`: Redownload even if up-to-date |
| `list`          | Show subscribed items                                        <br>`-v`: Display detailed file info |
| `remove <id>`   | Unsubscribe + delete files (cleans orphaned collection items) |
| `info`          | Display config, storage usage, and stats                      |
| `import <path>` | Import workshop IDs from `workshop_maps.txt` (NMRiH)          |
| `help`          | Show this command reference                                   |
| `exit`          | Exit                                                          |


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
## Notes

Inspired by the Workshop addon managers from [No More Room in Hell](https://store.steampowered.com/app/224260/No_More_Room_in_Hell/) and [Pirates, Vikings, and Knights II](https://store.steampowered.com/app/253210/Pirates_Vikings_and_Knights_II/) by [felis-catus](https://github.com/felis-catus)

