#include <sourcemod>
#pragma semicolon 1
#pragma newdecls required

public Plugin myinfo =
{
    name = "NecoDL",
    description = "",
    author = "Dysphie",
    version = "1.0.0",
    url = ""
};

ConVar cvMapFileID;

public void OnPluginStart()
{
    cvMapFileID = FindConVar("sv_workshop_map_id");
    if (!cvMapFileID) SetFailState("Could not find sv_workshop_map_id convar");
}

public void OnMapStart()
{
    char mapName[PLATFORM_MAX_PATH];
    GetCurrentMap(mapName, sizeof(mapName));

    KeyValues kv = new KeyValues("WorkshopMaps");
    if (!kv.ImportFromFile("workshop_maps.txt")) {
        LogError("Failed to open workshop_maps.txt");
        return;
    } 

    char workshopID[32];
    kv.GetString(mapName, workshopID, sizeof(workshopID));

    if (workshopID[0]) 
    {
        PrintToServer("[NecoDL] Mapping %s to %s", mapName, workshopID);
        cvMapFileID.SetString(workshopID);
    }
    else
    {
        PrintToServer("[NecoDL] Mapping %s to -1", mapName, workshopID);
    }

    delete kv;
}
