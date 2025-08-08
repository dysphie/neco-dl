# necodl

A standalone Workshop addon manager written in Rust for SRCDS.

I built this as a workaround for Steam API issues where valid workshop entries fail to return data, making the built-in workshop tools not work.

---

## Launching the tool

```sh
./necodl
```

---

## Commands

| command              | description                                                                                                                                                                                         |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `download <file_id>` | Subscribes to a Workshop file and downloads it. If it's a collection, all items are downloaded individually. Uses local cache to skip files already up to date. Add `--force` to force re-download. |
| `remove <file_id>`   | Unsubscribes from a Workshop file. If it's a collection, all included maps are removed (unless shared with another collection).                                                                     |
| `list`               | Lists all current Workshop subscriptions.                                                                                                                                                           |
| `update`             | Updates all existing Workshop subscriptions. Skips up-to-date files unless `--force` is used.                                                                                                       |

---

## Config

Example configuration file:

```toml
steam_cmd = "C:/steamcmd"            # path to steamcmd
download_dir = "./downloads"         # directory to move downloaded files

# For NMRiH only:
# List of workshop_maps.txt files to edit after download
# Needed so clients download the map on join
workshop_cfgs = [
    "nmrih/maps/workshop_maps.txt",
    "custom/maps/workshop_maps.txt"
]
```

## Notes

This project is in early use. Feedback and contributions are welcome!

