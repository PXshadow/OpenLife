package openlife.server;

import openlife.settings.ServerSettings;

// Holds all Saved Lineage  Information
// TODO load on server start
class Lineage 
{
    private static var AllLineages = new Map<Int,Lineage>();
    public static function AddLineage(lineageId:Int, lineage:Lineage)
    {
        lineage.myId = lineageId;
        AllLineages[lineage.myId] = lineage;
    }

    public var name = ServerSettings.StartingName;
    private var myFamilyName = ServerSettings.StartingFamilyName;

    // use Ids since not all might be available
    public var myId:Int = -1;
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

    public function getMotherLineage() : Lineage
    {
        return AllLineages[motherId];    
    }

    public function getFatherLineage() : Lineage
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

    // p_id mother_id grandmother_id great_grandmother_id ... eve_id eve=eve_id
    public function createLineageString() : String
    {
        var lineageString = '$myId';
        var tmpMotherLineage = this.getMotherLineage();

        for (ii in 0...10)
        {
            if(tmpMotherLineage == null) break;

            lineageString += ' ${tmpMotherLineage.myId}';

            tmpMotherLineage = tmpMotherLineage.getMotherLineage();
        }

        trace('Lineage: $lineageString');

        return lineageString;
    }
}