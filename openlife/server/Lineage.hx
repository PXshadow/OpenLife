package openlife.server;

import openlife.settings.ServerSettings;

// Holds all Saved Lineage  Information
class Lineage 
{
    public static var AllLineages = new Map<Int,Lineage>();

    public var name = ServerSettings.StartingName;
    private var myFamilyName = ServerSettings.StartingFamilyName;

    // use Ids since not all might be available
    public var motherId:Int = -1;
    public var fatherId:Int = -1;

    public function new(){}

    public var mother(get, set):GlobalPlayerInstance;

    public function get_mother()
    {
        return GlobalPlayerInstance.AllPlayers[motherId];
    }

    public function set_mother(newMother:GlobalPlayerInstance)
    {
        motherId = newMother.p_id;
        return newMother;
    }

    public var father(get, set):GlobalPlayerInstance;

    public function get_father()
    {
        return GlobalPlayerInstance.AllPlayers[fatherId];
    }

    public function set_father(newFather:GlobalPlayerInstance)
    {
        fatherId = newFather.p_id;
        return newFather;
    }

    public function getMotherLinage() : Lineage
    {
        return AllLineages[motherId];    
    }

    public function getFatherLinage() : Lineage
    {
        return AllLineages[fatherId];    
    }

    public var familyName(get, null):String;

    public function get_familyName()
    {
        return myFamilyName;
    }

    public function setFamilyName(newName:String)
    {
        return myFamilyName = newName; // TODO change  
    }
}