package openlife.server;

import sys.io.File;

class NamingHelper
{
    static var FemaleNames = new Map<String, Map<String, String>>();
    static var MaleNames = new Map<String, Map<String, String>>();

    static var FemaleNameCount = new Map<String,Int>();
    //static var FemaleNamesByIndex = new Map<String, String>();
    //static var MaleNamesByIndex = new Map<String, String>();

    public static function GetName(newName:String, female:Bool) : String
    {
        newName = newName.toUpperCase();
        var index = newName.substr(0,2);
        //trace('Name: index: $index');
        var map = female ? FemaleNames[index] : MaleNames[index];

        if(map == null) return null;
        var name = map[newName];

        if(name != null) return name;

        for(ii in 1...newName.length - 1)
        {
            var testName = newName.substr(0, newName.length - ii);

            //trace('Name: Test: $testName');

            for(n in map)
            {
                if(StringTools.startsWith(n, testName)) return n;
                if(StringTools.contains(n, testName)) return n;
            }
        }

        return null;
    }

    public static function ReadNames() : Bool
    {
        var reader = null;

        try{
            //var rtti = haxe.rtti.Rtti.getRtti(ServerSettings);
            var dir = './';
            reader = File.read(dir + "femaleNames.txt", false);

            var name = "";
            var count = 0;

            while(reader.eof() == false)
            {
                count++;

                name = reader.readLine();

                var index = name.substr(0,2);

                var map = FemaleNames[index];

                if(map == null)
                {
                    map = new Map<String, String>();
                    FemaleNames[index] = map;
                }

                map[name] = name;
                //var count2 = FemaleNameCount[index];
                //count2++;
                //FemaleNameCount[index] = count2;

                //trace('Name: $name index: $index $count / $count2');
            }
        }
        catch(ex)
        {
            if(reader != null) reader.close();

            var name = NamingHelper.GetName('Spoon', true);
            trace('Name: $name');

            var name = NamingHelper.GetName('SpoonWood', true);
            trace('Name: $name');

            var name = NamingHelper.GetName('Filea', true);
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
}