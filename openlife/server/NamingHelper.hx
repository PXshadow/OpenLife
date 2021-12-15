package openlife.server;

import openlife.settings.ServerSettings;
import openlife.server.GlobalPlayerInstance.Emote;
import openlife.client.ClientTag;
import sys.io.File;

using StringTools;

class NamingHelper
{
    static var FemaleNames = new Map<String, Map<String, String>>();
    static var MaleNames = new Map<String, Map<String, String>>();

    /*
    NM
    p_id first_name last_name
    p_id first_name last_name
    p_id first_name last_name
    ...
    p_id first_name last_name
    #


    Gives name of player p_id.

    last_name may be ommitted.
    */
    public static function DoNaming(p:GlobalPlayerInstance, text:String)        
    {
        //trace('TEST Naming1: $text');

        var doFamilyName = text.startsWith('I AM');
        
        if(doFamilyName == false && text.startsWith('YOU ARE') == false) return;

        var targetPlayer = doFamilyName ? p : p.heldPlayer;
        
        if(targetPlayer == null) targetPlayer = p.getClosestPlayer(5); // 5

        //trace('TEST Naming2: $text');

        if(targetPlayer == null) return;

        if(doFamilyName)
        {
            if(targetPlayer.familyName != ServerSettings.StartingFamilyName) return;
        }
        else if(targetPlayer.name != ServerSettings.StartingName) return;

        var strings = text.split(' ');

        if(strings.length < 3) return;
        
        var name = strings[2];

        if(name.length < 3) return;

        //var r = ~/^[a-z]+$/i; // only letters
        var r = ~/[^a-z]/i; // true if anything but letters
        if(r.match(name)) return; // return if there is anything but letters

        // TODO choose name from list
        
        trace('TEST Naming: $name');

        if(doFamilyName)
        {
            // check if name is used
            for(p in GlobalPlayerInstance.AllPlayers)
            {
                if(p.familyName == name)
                {
                    trace('family name: "$name" is used already!');

                    return;
                }
            }

            targetPlayer.lineage.setFamilyName(name); // check if used
        }
        else
        {
            // check if name is used
            for(c in Connection.getConnections())
            {
                if(c.player.name == name && c.player.familyName == p.familyName)
                {
                    trace('name: "$name" is used already!');

                    return;
                }
            }

            for(ai in Connection.getAis())
            {
                if(ai.player.name == name && ai.player.familyName == p.familyName)
                {
                    trace('name: "$name" is used already!');

                    return;
                }
            }

            targetPlayer.name = name;
        }

        trace('TEST Naming: ${targetPlayer.p_id} ${targetPlayer.name} ${targetPlayer.familyName}');
        
        if(doFamilyName)
        {
            // all family member names changed
            for(p in GlobalPlayerInstance.AllPlayers)
            {
                trace('FAMILYNAME: ${p.name} ${p.familyName}');

                if(p.familyName == name)
                {
                    for(c in Connection.getConnections())
                    {
                        c.send(ClientTag.NAME,['${p.p_id} ${p.name} ${p.familyName}']);
                    }
                }
            }

        }
        else
        {
            // only one name changed
            for(c in Connection.getConnections())
            {
                c.send(ClientTag.NAME,['${targetPlayer.p_id} ${targetPlayer.name} ${targetPlayer.familyName}']);
            }
        }

        p.doEmote(Emote.happy); // dont worry be happy!
        if(p != targetPlayer) targetPlayer.doEmote(Emote.happy); 
    }


    public static function GetName(newName:String, female:Bool) : String
    {
        newName = newName.toUpperCase();
        var index = newName.substr(0,2);
        var letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";

        //trace('Name: index: $index');
        
        for(i in 0...20)
        {
            // change the name little bit
            if(i > 0)
            {
                var oldName = newName;
                var newChar =  letters.charAt(WorldMap.calculateRandomInt(letters.length -1));
                index = index.charAt(0) + newChar;
                
                newName = '${newName.charAt(0)}$newChar${newName.substring(2, newName.length)}';

                trace('NAME: $oldName ==> $newName index: $index');
            }
            
            var map = female ? FemaleNames[index] : MaleNames[index];

            if(map == null)
            {
                continue;
            } 

            var name = map[newName];

            if(name != null && isUsedName(name) == false) return name;

            for(ii in 1...newName.length - 1)
            {
                var testName = newName.substr(0, newName.length - ii);

                //if(index == "SU") trace('Name: Test: $testName');

                for(n in map)
                {                
                    if(StringTools.startsWith(n, testName) || StringTools.contains(n, testName))
                    {
                        if(isUsedName(name) == false) return n;
                    }
                }
            }
        }

        return null;
    }

    private static function isUsedName(name:String) : Bool
    {
        for(p in GlobalPlayerInstance.AllPlayers)
        {
            if(p.name == name) return true;
        }    

        return false;
    }

    public static function ReadNames() : Bool
    {
        var result1 = ReadNamesByGender(true);
        var result2 = ReadNamesByGender(false);

        //Test();
      
        return result1 && result2; 
    }

    public static function ReadNamesByGender(female:Bool) : Bool
    {
        var reader = null;

        try{
            //var rtti = haxe.rtti.Rtti.getRtti(ServerSettings);
            var dir = './';
            dir += female ? "femaleNames.txt" : "maleNames.txt";
            var nameMap = female ? FemaleNames : MaleNames;        

            reader = File.read(dir, false);

            var name = "";
            var count = 0;
            

            while(reader.eof() == false)
            {
                count++;

                name = reader.readLine();

                var index = name.substr(0,2);

                var map = nameMap[index];

                if(map == null)
                {
                    map = new Map<String, String>();
                    nameMap[index] = map;
                }

                map[name] = name;

                //if(StringTools.startsWith(name, 'SU')) trace('Name: $name index: $index $count');
            }
        }
        catch(ex)
        {
            if(reader != null) reader.close();

            if('$ex' == 'Eof')
            {
                reader.close(); 
                return true;     
            }
                
            trace(ex);

            return false;
        }

        return true;     
    }

    public static function Test()
    {
        var name = NamingHelper.GetName('Suoon', false);
        trace('Name: $name');

        var name = NamingHelper.GetName('Spoon', false);
        trace('Name: $name');

        var name = NamingHelper.GetName('Spoon', false);
        trace('Name: $name');

        var name = NamingHelper.GetName('SpoonWood', false);
        trace('Name: $name');

        var name = NamingHelper.GetName('Filea', false);
        trace('Name: $name');

        var name = NamingHelper.GetName('natragsing', true);
        trace('Name: $name');

        var name = NamingHelper.GetName('Martin', true);
        trace('Name: $name');

        var name = NamingHelper.GetName('Jason', true);
        trace('Name: $name');

        var name = NamingHelper.GetName('Martina', true);
        trace('Name: $name');

        var name = NamingHelper.GetName('Alina', true);
        trace('Name: $name');

        var name = NamingHelper.GetName('xxx', true);
        trace('Name: $name');

        var name = NamingHelper.GetName('yyyZdsd', true);
        trace('Name: $name');

        var name = NamingHelper.GetName('yyyZdsd', false);
        trace('Name: $name');
    }
}