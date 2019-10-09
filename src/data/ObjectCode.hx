package data;

class ObjectCode
{
    private static var spring:Array<Int> = [3030,3031,3032,3033,3036,3037,3038,3039,3040,3041,3042,3044,1096,662,663,664,665,1861,2388,2389,2390];
    private static var rift:Array<Int> = [];
    public static function look(id:Int)
    {

    }
    public static function id(name:String):Array<Int>
    {
        //lower case and no spaces
        return switch(StringTools.replace(name.toLowerCase()," ",""))
        {
            case "milkweed":
            [50,51,52];
            case "bighardrock":
            [32];
            case "stone":
            [33];
            case "sharpstone":
            [34];
            //berry bush
            case "berrybush":
            [30];
            case "languishingberrybush":
            [392];
            case "dryberrybush":
            [393];
            case "berry" | "berries": 
            [31];
            case "emptyberrybush":
            [1135];
            case "tulereeds" | "rulereed" | "reed" | "reeds":
            [121];
            case "tule" | "tules":
            [123,124];
            case "basket":
            [292];
            case "food":
            [
                30,//Wild Gooseberry Bush
                31,//Gooseberry
                40,//Wild Carrot
                197,//Cooked Rabbit
                253,//Bowl of Gooseberries
                272,//Cooked Berry Pie
                273,//Cooked Carrot Pie
                274,//Cooked Rabbit Pie
                275,//Cooked Berry Carrot Pie
                276,//Cooked Berry Rabbit Pie
                277,//Cooked Rabbit Carrot Pie
                278,//Cooked Berry Carrot Rabbit Pie
                402,//Carrot
                518,//Cooked Goose
                570,//Cooked Mutton
                763,//Fruiting Barrel Cactus
                768,//Cactus Fruit
            ];
            case "trees" | "tree":
            [
                760, //Dead Tree
                527, //Willow Tree
                49, //Juniper Tree

                2452, //Yule Tree
                2454, //Spent Yule Tree
                2455, //Yule Tree with Half Candles

                406, //Yew Tree
                153, //Yew Tree #branch

                530, //Bald Cypress Tree

                2142, //Banana Tree 
                2145, //Empty Banana Tree

                48, //Maple Tree
                63, //Maple Tree #branch

                2135, //Rubber Tree
                2136, //Slashed Rubber Tree

                45, //Lombardy Poplar Tree
                65, //Lombardy Poplar Tree

                99, //White Pine Tree
                100, //White Pine Tree with Needles
                2450, //Pine Tree with Candles
                2461, //Pine Tree with Cardinals
                2434, //Pine Tree with One Gardland
                2436, //Pine Tree with Two Gardland

                1874, //Wild Mango Tree
                1875, //Fruiting Domestic Mango Tree
                1876, //Languishing Domestic Mango Tree
                1922, //Dry Fertile Domestic Mango Tree
                1923, //Wet Fertile Domestic Mango Tree
            ];
            case "watersource":
            [
                511, //Pound
                662, //Shallow Well
                660, //Full Bucket of Water
                1099, //Partial Buket Of Water
            ];
            case "watersourcebucket":
            [
                663, //Deep Well
                1097, //Full Deep Well
                706, //Ice Hole
            ];
            case "bucket":
            [
                659, //Empty Bucket
                660, //Full Bucket of Water
                1099, //Partial Buket Of Water
            ];
            case "danger":
            //thanks Hetuw (Whatever)
            [
                2156, // Mosquito swarm

                764,  // Rattle Snake
                1385, //Attacking Rattle Snake
                
                1328, //Wild board with Piglet
                1333, //Attacking Wild Boar
                1334, //Attacking Wild Boar with Piglet
                1339, //Domestic Boar
                1341, //Domestic Boar with Piglet
                1347, //Attacking Boar# domestic
                1348, //Attacking Boar with Piglet# domestic

                418, //Wolf
                1630, //Semi-tame Wolf
                420, //Shot Wolf
                428, //Attacking Shot Wolf
                429, //Dying Semi-tame Wolf
                1761, //Dying Semi-tame Wolf
                1640, //Semi-tame Wolf# just fed
                1642, //Semi-tame Wolf# pregnant
                1636, //Semi-tame Wolf with Puppy#1
                1635, //Semi-tame Wolf with Puppy#2
                1631, //Semi-tame Wolf with Puppy#3
                1748, //Old Semi-tame Wolf
                //1641, //@ Deadly Wolf

                628, //Grizzly Bear
                655, //Shot Grizzly Bear#2 attacking
                653, //Dying Shot Grizzly Bear #3
                631, //Hungry Grizzly Bear
                646, //@ Unshot Grizzly Bear
                635, //Shot Grizzly Bear#2
                645, //Shot Grizzly Bear
                632, //Shot Grizzle Bear#1
                637, //Shot Grizzle Bear#3
                654, //Shot Grizzle Bear#1 attacking

                1789, //Abused Pit Bull
                1747, //Mean Pit Bull
                1712, //Attacking Pit Bull
            ];
            default: 
            [-1];
        }
    }
}