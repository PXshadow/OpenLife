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
    public var myEveId:Int = -1; // TODO support family head
    public var motherId:Int = -1;
    public var fatherId:Int = -1;

    public function new(){}

    public var eve(get, null):GlobalPlayerInstance;

    public function get_eve()
    {
        return GlobalPlayerInstance.AllPlayers[myEveId];
    }

    public var eveLineage(get, null):Lineage;

    public function get_eveLineage()
    {
        return AllLineages[myEveId];
    }


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
        return this.eveLineage.myFamilyName; 
    }

    // TODO support own family name with ditance X from last and prestiege Y
    public function setFamilyName(newName:String) 
    {
        //trace('setFamilyName: $familyName ==> $newName');
        return this.eveLineage.myFamilyName = newName;
    }

    // p_id mother_id grandmother_id great_grandmother_id ... eve_id eve=eve_id
    public function createLineageString() : String
    {
        var lineageString = '$myId';

        if(myId == myEveId) return lineageString;

        var tmpMotherLineage = this.getMotherLineage();
        var addedEve = false;

        for (ii in 0...10)
        {
            if(tmpMotherLineage == null) break;

            lineageString += ' ${tmpMotherLineage.myId}';

            if(tmpMotherLineage.myId == myEveId)
            {
                addedEve = true;
                break;
            } 

            tmpMotherLineage = tmpMotherLineage.getMotherLineage();
        }

        // if lineage too long add "eve_id eve=eve_id"
        if(addedEve == false)
        {
            lineageString += ' eve_id=$myEveId'; // TODO test
        }

        trace('Lineage: ${lineageString}');

        return lineageString;
    }
}