package openlife.server;

import openlife.settings.ServerSettings;

@:enum abstract PrestigeClass(Int) from Int to Int
{
    public var NotSet = 0; 
    public var Serf = 1;  
    public var Commoner = 2;  
    public var Noble = 3;  
    public var King = 6; 
    public var Emperor = 7; 
}

// Holds all Saved Lineage  Information
// TODO load on server start
class Lineage 
{
    private static var PrestigeClasses = ['Not Set', 'Serf', 'Commoner','Noble', 'Noble', 'Noble', 'King', 'Emperor'];

    private static var AllLineages = new Map<Int,Lineage>();
    public static function AddLineage(lineageId:Int, lineage:Lineage)
    {
        lineage.myId = lineageId;
        AllLineages[lineage.myId] = lineage;
    }

    public static function GetLineage(lineageId:Int) : Lineage
    {
        return AllLineages[lineageId];
    }

    public var name = ServerSettings.StartingName;
    private var myFamilyName = ServerSettings.StartingFamilyName;

    // use Ids since not all might be available
    public var myId:Int = -1;
    public var po_id:Int = -1;
    public var deathTime:Float;
    public var age:Float;
    public var trueAge:Float;
    public var deathReason:String;

    public var myEveId:Int = -1; // TODO support family head
    public var motherId:Int = -1;
    public var fatherId:Int = -1;
    public var prestigeClass:PrestigeClass = PrestigeClass.Commoner;

    public function new(player:GlobalPlayerInstance)
    {
        this.deathTime = TimeHelper.tick;
        this.myId = player.p_id;
        this.po_id = player.po_id;
    }

    public var className(get, null):String;

    public function get_className()
    {
        return PrestigeClasses[this.prestigeClass];
    }

    public function getFullName(withUnderscore:Bool = false, ignoreFirstName = false)
    {
        var fullName = ignoreFirstName ? '${this.familyName} ${this.className}' : '${this.name} ${this.familyName} ${this.className}';
        
        if(withUnderscore) return StringTools.replace(fullName, ' ', '_');

        return fullName;
    }

    public function getDeadSince() : Int
    {
        var years = TimeHelper.tick - this.deathTime;
        years *= TimeHelper.tickTime; // seconds
        years /= 60; // years
        return Math.floor(years);
    }

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
    public function createLineageString(withMe:Bool = false) : String
    {
        var lineageString = withMe ? '$myId' : '';

        if(myId == myEveId) return lineageString;

        var tmpMotherLineage = this.getMotherLineage();
        var addedEve = false;

        for (ii in 0...10)
        {
            if(tmpMotherLineage == null) break;
            if(lineageString.length > 0) lineageString += ' ';
            lineageString += '${tmpMotherLineage.myId}';

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