
<img height="128" alt="necoDL icon" src="https://github.com/user-attachments/assets/65c94234-faca-4b64-9487-e91cf232f543" />

# NecoDL

A CLI Workshop addon manager for Source Engine Dedicated Servers.

I built this as a workaround for Steam API issues where valid workshop entries fail to return data, making the built-in workshop tools not work.

---

## Usage

* Grab the latest binary from the [releases](https://github.com/dysphie/neco-dl/releases) page.
* Adjust the settings in the [config](https://github.com/dysphie/neco-dl?tab=readme-ov-file#config) file.
* Launch the tool in the terminal with: `./necodl`

---

## Commands

| command              | description                                                                                                                                                                                         |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `download <file_id>` | Subscribes to a Workshop file and downloads it. If it's a collection, all items are downloaded individually. |
| `remove <file_id>`   | Unsubscribes from a Workshop file. If it's a collection, all included items are removed (unless shared with another collection)                          |
| `list`               | Lists all current Workshop subscriptions. Use `-v` for detailed info.                                                                                                                               |
| `update`             | Updates all existing Workshop subscriptions.                                                                                               |
| `generate`           | (NMRiH) Generates `workshop_maps.txt` files from your current subscriptions.                                                                                                                             |
| `info`               | Shows configuration details and current status information.                                                                                                                                         |
| `help`               | Displays the list of available commands and their descriptions.                                                                                                                                     |
| `exit`               | Exits the application.                                                                                                                                                                              |

---

## Config

Example configuration file:

```toml
steam_cmd = "./steamcmd/steamcmd.exe" # Path to SteamCMD (.exe or .sh)
download_dir = "./downloads" # Where to place downloaded files after download
appid = "224260"

# For NMRiH only:
# List of workshop_maps.txt files to edit after download
# Needed so clients download the map on join
workshop_cfgs = [
    "./nmrihserver1/workshop_maps.txt",
    "./nmrihserver2/workshop_maps.txt"
]
```

> \[!TIP]
> Instead of setting your download folder to your server's maps folder, you can dynamically mount it by adding `game+mod <path>` to your `gameinfo.txt`.

