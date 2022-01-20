package openlife.server;

import haxe.Exception;
import sys.io.File;
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

    public var myId:Int = -1;
    public var accountId:Int;

    public var name = ServerSettings.StartingName;
    private var myFamilyName = ServerSettings.StartingFamilyName;

    // use Ids since not all might be available
    
    public var po_id:Int = -1;
    public var birthTime:Float;
    public var deathTime:Float;
    public var age:Float;
    public var trueAge:Float;

    public var deathReason:String;
    public var lastSaid:String;
    public var prestige:Float;
    public var coins:Float;

    public var myEveId:Int = -1; // TODO support family head
    public var motherId:Int = -1;
    public var fatherId:Int = -1;

    public var prestigeClass:PrestigeClass = PrestigeClass.Commoner;

    public static function WriteAllLineages(path:String)
    { 
        var count = 0;
        var dataVersion = 1;
        var writer = File.write(path, true);

        for(linage in AllLineages) count++;

        writer.writeInt32(dataVersion); 
        writer.writeInt32(count);
        
        for(linage in AllLineages)
        {
            writer.writeInt32(linage.myId);
            writer.writeInt32(linage.accountId);

            writer.writeString('${linage.name}\n');
            writer.writeString('${linage.familyName}\n'); // writes eve family name

            writer.writeInt32(linage.po_id);
            writer.writeDouble(linage.birthTime);
            writer.writeDouble(linage.deathTime);
            writer.writeFloat(linage.age);
            writer.writeFloat(linage.trueAge);

            writer.writeString('${linage.deathReason}\n');
            writer.writeString('${linage.lastSaid}\n');
            writer.writeFloat(linage.prestige);
            writer.writeFloat(linage.coins);

            writer.writeInt32(linage.myEveId);
            writer.writeInt32(linage.motherId);
            writer.writeInt32(linage.fatherId);

            writer.writeInt8(linage.prestigeClass);
        }

        writer.close();

        if(ServerSettings.DebugWrite) trace('wrote $count Lineages...');
    }

    public static function ReadLineages(path:String) 
    {
        var reader = File.read(path, true);
        var expectedDataVersion = 1;
        var dataVersion = reader.readInt32();
        var count = reader.readInt32();
        AllLineages = new Map<Int,Lineage>();

        trace('Read from file: $path count: ${count}');

        if(dataVersion != expectedDataVersion) throw new Exception('ReadLineages: Data version is: $dataVersion expected data version is: $expectedDataVersion');

        try{
            for(i in 0...count)
            {
                var linage = new Lineage(null);

                linage.myId = reader.readInt32();
                linage.accountId = reader.readInt32();
    
                linage.name = reader.readLine();
                linage.familyName = reader.readLine();

                linage.po_id = reader.readInt32();
                linage.birthTime = reader.readDouble();
                linage.deathTime = reader.readDouble();
                linage.age = reader.readFloat();
                linage.trueAge = reader.readFloat();

                linage.deathReason = reader.readLine();
                linage.lastSaid = reader.readLine();
                linage.prestige = reader.readFloat();
                linage.coins = reader.readFloat();

                linage.myEveId = reader.readInt32();
                linage.motherId = reader.readInt32();
                linage.fatherId = reader.readInt32();

                linage.prestigeClass = reader.readInt8();

                AllLineages[linage.myId] = linage;
            }
        }
        catch(ex)
        {
            reader.close();
            throw ex;
        }

        reader.close();

        //trace('read $count Lineages...');
    }

    public function new(player:GlobalPlayerInstance)
    {
        if(player == null) return;

        this.birthTime = TimeHelper.tick;
        this.myId = player.p_id;
        this.po_id = player.po_id;
        this.accountId = player.account.id;

        //trace('accountId: ${this.accountId}');
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