package openlife.server;

import sys.io.File;

class NamingHelper
{
    static var FemaleNames = new Map<String, Map<String, String>>();
    static var MaleNames = new Map<String, Map<String, String>>();

    //static var FemaleNameCount = new Map<String,Int>();


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