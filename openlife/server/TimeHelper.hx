package openlife.server;

import openlife.data.object.ObjectHelper;
import openlife.settings.ServerSettings;

class TimeHelper
{
    private static var tickTime = 1 / 20;

    //private static var TimeHelper = new TimeHelper();

    public static var tick:Int = 0;
    private static var lastTick:Int = 0;
    private static var serverStartingTime:Float = Sys.time();

    // TODO currently not needed, since for all objects on the map every second all time objects are generated
    //public var timeObjectHelpers:Array<ObjectHelper>; 
    private static var worldMapTimeStep = 0; // counts the time steps for doing map time stuff, since some ticks may be skiped because of server too slow

    //private function new(){}

    public static function CalculateTimeSinceTicksInSec(ticks:Int):Float
    {
        return (TimeHelper.tick - ticks) * TimeHelper.tickTime;
    }

    public static function DoTimeLoop()
    {
        while (true)
        {
            @:privateAccess haxe.MainLoop.tick();
            @:privateAccess TimeHelper.DoTimeStuff();
            TimeHelper.tick++;
            Sys.sleep(TimeHelper.tickTime);
        }
    }

    public static function DoTimeStuff()
    {
        //if(TimeHelper.serverStartingTime <= 0) serverStartingTime = Sys.time();

        var timeSinceStart = Sys.time() - TimeHelper.serverStartingTime;
        var timePassedInSeconds = CalculateTimeSinceTicksInSec(lastTick);

        TimeHelper.lastTick = tick;

        // never skip a time task tick that is every 20 ticks
        // TODO what to do if server is too slow?
        //if(TimeHelper.tick % 20 != 0 && TimeHelper.tick * TimeHelper.tickTime < timeSinceStart - TimeHelper.tickTime) TimeHelper.tick += 1;
        if((TimeHelper.tick + 1) * TimeHelper.tickTime < timeSinceStart) TimeHelper.tick += 1;

        if(TimeHelper.tick % 200 == 0) trace('Time: ${TimeHelper.tick * TimeHelper.tickTime} TimeSinceStart: $timeSinceStart');

        Server.server.map.mutex.acquire(); // TODO add try catch for non debug

        for (connection in Server.server.connections)
        {
            updateAge(connection, timePassedInSeconds);

            updateFood(connection, timePassedInSeconds);

            MoveExtender.updateMovement(connection.player);
        }

        DoWorldMapTimeStuff();

        Server.server.map.mutex.release();

        /* TODO currently it goes through the hole map each sec / this may later not work
        for(helper in this.map.timeObjectHelpers){
            var passedTime = calculateTimeSinceTicksInSec(helper.creationTimeInTicks);
            if(passedTime >= helper.timeToChange)
            {
                trace('TIME: ${helper.objectData.description} passedTime: $passedTime neededTime: ${helper.timeToChange}');       

                TransitionHelper.doTimeTransition(helper);
            }
        }*/
    }

    private static function updateAge(c:Connection, timePassedInSeconds:Float)
    {
        var tmpAge = c.player.age;

        c.player.age += timePassedInSeconds / c.player.age_r;
        
        if(Std.int(tmpAge) != Std.int(c.player.age))
        {
            //trace('update age');
            //c.player.po_id += 1;
            Connection.SendUpdateToAllClosePlayers(c.player, false);
        }
    }

    private static function updateFood(c:Connection, timePassedInSeconds:Float)
    {
        //trace('food_store: ${connection.player.food_store}');

        var tmpFood = Math.ceil(c.player.food_store);
        var tmpExtraFood = Math.ceil(c.player.yum_bonus);
        var foodDecay = timePassedInSeconds * ServerSettings.FoodUsePerSecond; 

        if(c.player.yum_bonus > 0)
        {
            c.player.yum_bonus -= foodDecay;
        }
        else
        {
            c.player.food_store -= foodDecay;
        }

        if(tmpFood != Math.ceil(c.player.food_store) || tmpExtraFood != Math.ceil(c.player.yum_bonus))
        {
            c.player.sendFoodUpdate(false);
            c.send(FRAME, null, false);
        }
    }

    public static function DoWorldMapTimeStuff()
    {
        // devide in X steps
        var timeParts = 20; 
        var worldMap = Server.server.map;

        //var partSize = Std.int(length / timeParts);
        //var start = (worldMapTimeStep % timeParts) * partSize;
        //var end = start + partSize;

        var partSizeY = Std.int(worldMap.height / timeParts);
        var startY = (worldMapTimeStep % timeParts) * partSizeY;
        var endY = startY + partSizeY;

        //trace('startY: $startY endY: $endY worldMap.height: ${worldMap.height}');

        worldMapTimeStep++;

        for (y in startY...endY)
        {
            for(x in 0...worldMap.width)
            {
                var obj = worldMap.getObjectId(x,y);

                if(obj[0] == 0) continue;     

                var helper = worldMap.getObjectHelper(x,y,true); 

                if(helper != null)
                {              
                    // clear up not needed ObjectHelpers to save space
                    if(worldMap.deleteObjectHelperIfUseless(helper)) continue;

                    if(helper.timeToChange == 0) continue;

                    var passedTime = TimeHelper.CalculateTimeSinceTicksInSec(helper.creationTimeInTicks);
                    var timeToChange = helper.timeToChange;

                    if(passedTime >= timeToChange)
                    {
                        //trace('TIME: ${helper.objectData.description} passedTime: $passedTime neededTime: ${timeToChange}');       

                        // TODO maybe better not delete by default...
                        worldMap.setObjectHelperNull(x,y);
                        
                        TransitionHelper.doTimeTransition(helper);
                    }

                    continue;
                }

                var timeTransition = Server.transitionImporter.getTransition(-1, obj[0], false, false);
                if(timeTransition == null) continue;

                helper = worldMap.getObjectHelper(x,y); 
                helper.timeToChange = ObjectHelper.calculateTimeToChange(timeTransition);

                worldMap.setObjectHelper(x,y,helper);

                //trace('TIME: ${helper.objectData.description} neededTime: ${timeToChange}');  
                
                //var testObj = getObjectId(helper.tx, helper.ty);

                //trace('testObj: $testObj obj: $obj ${helper.tx},${helper.ty} i:$i index:${index(helper.tx, helper.ty)}');
            }
        }
    }  
}