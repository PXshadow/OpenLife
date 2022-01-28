package openlife.server;

import openlife.server.GlobalPlayerInstance.Emote;
import openlife.server.Biome.BiomeMapColor;
import openlife.auto.AiHelper;
import openlife.data.transition.TransitionImporter;
import haxe.Exception;
import openlife.macros.Macro;
import openlife.client.ClientTag;
import openlife.data.object.ObjectData;
import openlife.server.Biome.BiomeTag;
import openlife.data.transition.TransitionData;
import openlife.data.object.ObjectHelper;
import openlife.settings.ServerSettings;


@:enum abstract Seasons(Int) from Int to Int
{
    public var Spring = 0; 
    public var Summer = 1; 
    public var Autumn = 2; 
    public var Winter = 3; 
}

class TimeHelper
{
    public static var tickTime = 1 / 20;
    public static var tick:Float = 0;       // some are skipped if server is too slow
    //public static var allTicks:Float = 0;   // these ticks will not be skipped, but can be slower then expected

    public static var lastTick:Float = 0;
    private static var serverStartingTime:Float;

    // Time Step Stuff
    private static var worldMapTimeStep = 0; // counts the time steps for doing map time stuff, since some ticks may be skiped because of server too slow
    private static var TimeTimeStepsSartedInTicks:Float = 0;
    private static var TimePassedToDoAllTimeSteps:Float = 0;
    private static var WinterDecayChance:Float = 0; 
    private static var SpringRegrowChance:Float = 0;

    // Seaons
    private static var TimeToNextSeasonInYears:Float = ServerSettings.SeasonDuration;
    private static var TimeSeasonStartedInTicks:Float = 0;
    private static var Season:Seasons = Seasons.Spring;
    private static var SeasonNames = ["Spring", "Summer", "Autumn", "Winter"];
    private static var SeasonTemperatureImpact:Float = 0;
    private static var SeasonHardness:Float = 1;

    public static function CalculateTimeSinceTicksInSec(ticks:Float):Float
    {
        return (TimeHelper.tick - ticks) * TimeHelper.tickTime;
    }

    public static function DoTimeLoop()
    {
        serverStartingTime = Sys.time();
        var averageSleepTime:Float = 0.0;
        var skipedTicks = 0;
        var timeSinceStartCountedFromTicks:Float = TimeHelper.tick * TimeHelper.tickTime;
        serverStartingTime -= timeSinceStartCountedFromTicks;  // pretend the server was started before to be aligned with ticks

        //trace('Server Startign time: sys.time: ${Sys.time()} serverStartingTime: $serverStartingTime timeSinceStartCountedFromTicks: $timeSinceStartCountedFromTicks');

        DoTest();

        while (true)
        {
            if(ServerSettings.useOneGlobalMutex) WorldMap.world.mutex.acquire(); 

            TimeHelper.tick = Std.int(TimeHelper.tick + 1);

            var timeSinceStart:Float = Sys.time() - TimeHelper.serverStartingTime;
            timeSinceStartCountedFromTicks = TimeHelper.tick * TimeHelper.tickTime;
            
            // TODO what to do if server is too slow?
            if(TimeHelper.tick % 10 != 0  && timeSinceStartCountedFromTicks < timeSinceStart)
            {
                TimeHelper.tick = Std.int(TimeHelper.tick + 1);
                skipedTicks++;
            }
            if(TimeHelper.tick % 200 == 0)
            {
                averageSleepTime = Math.ceil(averageSleepTime / 200 * 1000) / 1000;
                //trace('Ticks: ${TimeHelper.tick}');
                trace('Connections: ${Connection.getConnections().length} AIs: ${Connection.getAis().length} Time Counted From Ticks: ${timeSinceStartCountedFromTicks} Time: ${Math.ceil(timeSinceStart)} Skiped Ticks: $skipedTicks Average Sleep Time: $averageSleepTime');
                averageSleepTime = 0;
                skipedTicks = 0;

                ServerSettings.readFromFile(false);

                //if(Server.server.connections.length > 0) Server.server.connections[0].player.doDeath();
            }

            @:privateAccess haxe.MainLoop.tick();

            Macro.exception(TimeHelper.DoTimeStuff());
            
            timeSinceStart = Sys.time() - TimeHelper.serverStartingTime;

            if(ServerSettings.useOneGlobalMutex) WorldMap.world.mutex.release(); 

            if(timeSinceStartCountedFromTicks > timeSinceStart)
            {
                var sleepTime = timeSinceStartCountedFromTicks - timeSinceStart;
                averageSleepTime += sleepTime;

                //trace('sleep: ${sleepTime}');
                Sys.sleep(sleepTime);
            }   
        }
    }    

    public static function DoTimeStuff()
    {
        var timePassedInSeconds = CalculateTimeSinceTicksInSec(lastTick);

        TimeHelper.lastTick = tick;  

        DoSeason(timePassedInSeconds);

        if(ServerSettings.useOnePlayerMutex) GlobalPlayerInstance.AllPlayerMutex.acquire(); // TODO if no global player mutex is used

        for (c in Connection.getConnections())
        {            
            if(DoTimeStuffForPlayer(c.player, timePassedInSeconds) == false) continue;

            if(TimeHelper.tick % 90 == 0) Macro.exception(c.sendToMeAllClosePlayers(false, false));
        }
        
        for (ai in Connection.getAis())
        {
            if(ai.player.deleted) ai.doRebirth(timePassedInSeconds);
            if(DoTimeStuffForPlayer(ai.player, timePassedInSeconds) == false) continue;
        }

        for (ai in Connection.getAis())
        {
            Macro.exception(ai.doTimeStuff(timePassedInSeconds));
        }

        Macro.exception(DoWorldMapTimeStuff()); // TODO currently it goes through the hole map each sec / this may later not work

        Macro.exception(RespawnObjects());

        Macro.exception(DecayObjects());

        var worldMap = Server.server.map; 
 
        // make sure they are not all at same tick!
        if((tick + 20) % ServerSettings.TicksBetweenSaving  == 0) Macro.exception(worldMap.updateObjectCounts());
        if(ServerSettings.saveToDisk && tick % ServerSettings.TicksBetweenSaving == 0) Macro.exception(Server.server.map.writeToDisk(false));
        if(ServerSettings.saveToDisk && (tick + 60) % ServerSettings.TicksBetweenBackups == Math.ceil(ServerSettings.TicksBetweenBackups / 2)) Macro.exception(Server.server.map.writeBackup());

        if(ServerSettings.useOnePlayerMutex) GlobalPlayerInstance.AllPlayerMutex.release(); // TODO mutext if no global player mutex is used

        DoTimeTestStuff();    
    }

    private static function DoSeason(timePassedInSeconds:Float)
    {
        var passedSeasonTime = TimeHelper.CalculateTimeSinceTicksInSec(TimeSeasonStartedInTicks);
        var timeToNextSeasonInSec = TimeToNextSeasonInYears * 60; 
        var tmpSeasonTemperatureImpact:Float = 0;
        
        tmpSeasonTemperatureImpact = ServerSettings.AverageSeasonTemperatureImpact * SeasonHardness;

        if(Season == Seasons.Spring || Season == Seasons.Autumn) tmpSeasonTemperatureImpact *= 0.25;
        if(Season == Seasons.Winter || Season == Seasons.Autumn) tmpSeasonTemperatureImpact *= -1;

        var factor = TimeToNextSeasonInYears * 15 * (1 / timePassedInSeconds);

        SeasonTemperatureImpact = (SeasonTemperatureImpact * factor + tmpSeasonTemperatureImpact) / (factor + 1);


        //if(tick % 20 == 0) trace('SEASON: ${SeasonHardness} TemperatureImpact: $SeasonTemperatureImpact tmp: $tmpSeasonTemperatureImpact');

        if(passedSeasonTime > timeToNextSeasonInSec)
        {
            var tmpSeasonHardness = SeasonHardness;

            TimeSeasonStartedInTicks = tick;
            TimeToNextSeasonInYears = ServerSettings.SeasonDuration / 2 + WorldMap.calculateRandomFloat() * ServerSettings.SeasonDuration;
            Season = (Season + 1) % 4;
            SeasonHardness = WorldMap.calculateRandomFloat() + 0.5;
            

            var seasonName = SeasonNames[Season];
            var message = 'SEASON: ${seasonName} is there! hardness: $SeasonHardness years: ${passedSeasonTime / 60} timeToNextSeasonInSec: $timeToNextSeasonInSec';
            
            trace(message);

            var hardSeason = (Season == Seasons.Winter || Season == Seasons.Summer) && SeasonHardness > 1.25;
            var hardText = hardSeason ? 'A hard ' : '';
            if(hardSeason && SeasonHardness > 1.4)
            {
                SeasonHardness += 0.1; // make it even harder
                hardText = 'A very hard ';
            }

            if(hardSeason) SeasonHardness = Math.pow(SeasonHardness, 2);

            // use same hardness for Spring as for winter. bad winter ==> good spring
            if(Season == Seasons.Spring) SeasonHardness = tmpSeasonHardness;

            TimeToNextSeasonInYears *= SeasonHardness;

            Connection.SendGlobalMessageToAll('$hardText ${seasonName} is comming!');            
        }
    }

    private static function DoTimeStuffForPlayer(player:GlobalPlayerInstance, timePassedInSeconds:Float) : Bool
    {
        if(player.deleted) return false; // maybe remove?

        Macro.exception(player.connection.doTime(timePassedInSeconds));        

        Macro.exception(UpdatePlayerStats(player, timePassedInSeconds));

        Macro.exception(updateAge(player, timePassedInSeconds));

        Macro.exception(updateFoodAndDoHealing(player, timePassedInSeconds));            

        Macro.exception(MoveHelper.updateMovement(player));

        Macro.exception(DoTimeOnPlayerObjects(player, timePassedInSeconds));

        if(TimeHelper.tick % 20 == 0) Macro.exception(updateTemperature(player));

        if(TimeHelper.tick % 30 == 0) Macro.exception(UpdateEmotes(player));
                
        return true;
    }

    private static function UpdatePlayerStats(player:GlobalPlayerInstance, timePassedInSeconds:Float)
    {
        
        //if(player.angryTime < 0 && player.angryTime > -1) player.angryTime = 0;

        //var moreAngry = player.isHoldingWeapon() || (player.lastPlayerAttackedMe != null && player.lastPlayerAttackedMe.isHoldingWeapon());
        var moreAngry = player.killMode || (player.lastPlayerAttackedMe != null && player.lastPlayerAttackedMe.isHoldingWeapon());

        if(moreAngry)
        {
            if(player.angryTime > -ServerSettings.CombatAngryTimeBeforeAttack ) player.angryTime -= timePassedInSeconds;
        } 
        else
        {
            if(player.angryTime < ServerSettings.CombatAngryTimeBeforeAttack ) player.angryTime += timePassedInSeconds;
        } 

        // if last attacker is far away set null
        if(player.lastPlayerAttackedMe != null)
        {
            var quadDist = AiHelper.CalculateDistanceToPlayer(player, player.lastPlayerAttackedMe);
            if(quadDist > 100)  player.lastPlayerAttackedMe = null;
        }

        // add new follower
        if(player.newFollowerTime > 0) player.newFollowerTime -= timePassedInSeconds;
        else
        {
            if(player.newFollower != null)
            {
                var exileLeader = player.newFollower.isExiledByAnyLeader(player);
                var notExiled = exileLeader == null;

                if(notExiled && player.newFollower.followPlayer != player.newFollowerFor)
                {
                    player.newFollower.followPlayer = player.newFollowerFor;
                    Connection.SendFollowingToAll(player.newFollower);

                    player.newFollower.connection.sendGlobalMessage('You follow now ${player.newFollowerFor.name} ${player.newFollowerFor.familyName}');

                    //player.newFollower.say('now I follow ${player.newFollowerFor.name}');

                    //player.say('He follows now ${player.newFollowerFor.name}');
                }

                player.newFollowerFor.newFollower = null;
                player.newFollowerFor.newFollowerFor = null;
                player.newFollower = null;
                player.newFollowerFor = null;       
            }
        }
    }

    private static function DoTimeOnPlayerObjects(player:GlobalPlayerInstance, timePassedInSeconds:Float)
    {
        var obj = player.heldObject;

        if(player.o_id[0] < 1) return;
        if(obj.timeToChange <= 0) return;
        
        obj.timeToChange -= timePassedInSeconds;

        if(obj.timeToChange <= 0)
        {
            var transition = TransitionImporter.GetTransition(-1, obj.parentId, false, false);

            //var desc = obj.objectData.description;
            // use alternative outcome for example for wound on player vs on ground
            var alternativeTimeOutcome = obj.objectData.alternativeTimeOutcome; 
            obj.id = alternativeTimeOutcome >=0 ? alternativeTimeOutcome : transition.newTargetID;

            //trace('TIME: ${desc} --> ${obj.objectData.description} transition: ${transition.newTargetID} alternative: ${obj.objectData.alternativeTimeOutcome} passedTime: $passedTime neededTime: ${timeToChange}'); 

            obj.creationTimeInTicks = TimeHelper.tick;

            player.setHeldObject(obj);

            player.setHeldObjectOriginNotValid(); // no animation
        }

        // TODO contained objects
        // TODO clothing decay / contained objects --> like in backpack
    }

    private static function UpdateEmotes(player:GlobalPlayerInstance)
    {
        //var temperatureMail = Math.pow(((player.heat - 0.5) * 10), 2) / 10;

        //trace('temperatureMail: $temperatureMail');

        if(player.isWounded())
        {
            Connection.SendEmoteToAll(player, Emote.shock);
            return;
        }

        //if(player.isHoldingWeapon() && player.angryTime < ServerSettings.CombatAngryTimeBeforeAttack / 2 )
        if(player.angryTime < 2 )
        {
            if(player.isHoldingWeapon()) player.doEmote(Emote.murderFace);
            else
            { 
                var lastPlayerAttackedMe = player.lastPlayerAttackedMe;
                if(lastPlayerAttackedMe != null && lastPlayerAttackedMe.lastAttackedPlayer == player && lastPlayerAttackedMe.isHoldingWeapon()) player.doEmote(Emote.terrified);
                else player.doEmote(Emote.angry);
            }

            return;
        }

        if(player.food_store < 0 && player.age >= ServerSettings.MinAgeToEat)
        {
            player.doEmote(Emote.starving);
            return;
        }

        if(player.angryTime < ServerSettings.CombatAngryTimeBeforeAttack)
        {
            if(player.isHoldingWeapon()) player.doEmote(Emote.angry);
            else
            { 
                if(player.lastPlayerAttackedMe != null && player.lastPlayerAttackedMe.isHoldingWeapon()) player.doEmote(Emote.shock);
                else player.doEmote(Emote.angry);
            }
        }

        if(player.isSuperHot()) player.doEmote(Emote.heatStroke);
        if(player.isSuperCold()) player.doEmote(Emote.pneumonia);  
        //else if(playerHeat > 0.6) player.doEmote(Emote.dehydration);
        
        if(player.isHoldingChildInBreastFeedingAgeAndCanFeed())
        {
            player.heldPlayer.doEmote(Emote.happy);
        }

        if(player.mother != null)
            {
                //if(this.isAi() == false) this.connection.sendMapLocation(this.mother,'MOTHER', 'mother');
                //if(player.isAi() == false) player.connection.sendMapLocation(player.mother,'MOTHER', 'leader');
                //if(player.mother.isAi() == false) player.mother.connection.sendMapLocation(player,'BABY', 'baby');
            }
    }

    private static function updateAge(player:GlobalPlayerInstance, timePassedInSeconds:Float)
    {
        var tmpAge = player.age;
        var aging = timePassedInSeconds / player.age_r;

        //trace('aging: ${aging}');

        //trace('player.age_r: ${player.age_r}');

        var healthFactor = player.CalculateHealthAgeFactor();
        var agingFactor:Float = 1;    
        
        //trace('healthFactor: ${healthFactor}');

        if(player.age < ServerSettings.GrownUpAge)
        {
            agingFactor = healthFactor;
        }
        else
        {
            agingFactor = 1 / healthFactor;
        }

        if(player.food_store < 0)
        {
            if(player.age < ServerSettings.GrownUpAge)
            {
                agingFactor *= ServerSettings.AgingFactorWhileStarvingToDeath;
            } 
            else
            {
                agingFactor *= 1 / ServerSettings.AgingFactorWhileStarvingToDeath;
            }
        }

        player.age_r = ServerSettings.AgingSecondsPerYear * agingFactor;

        player.trueAge += aging;

        aging *= agingFactor;

        player.age += aging;

        //trace('player.age: ${player.age}');
        
        if(Std.int(tmpAge) != Std.int(player.age))
        {
            trace('Player: ${player.p_id} Old Age: $tmpAge New Age: ${player.age} TrueAge: ${player.trueAge} agingFactor: $agingFactor healthFactor: $healthFactor');

            //player.yum_multiplier -= ServerSettings.MinHealthPerYear; // each year some health is lost
            player.food_store_max = player.calculateFoodStoreMax();

            // decay some coins per year
            if(player.coins > 100)
            {
                var decayedCoins:Float = Std.int(player.coins / 100); // 0,1% per year or 10% in 100 years
                decayedCoins = Math.min(decayedCoins, ServerSettings.MaxCoinDecayPerYear);
                player.coins -= decayedCoins;
                player.addPrestige(-decayedCoins);
            } 

            if(player.age > 60) player.doDeath('reason_age');

            //trace('update age: ${player.age} food_store_max: ${player.food_store_max}');
            player.sendFoodUpdate(false);

            if(player.isMoving() == false) Connection.SendUpdateToAllClosePlayers(player, false);

            if(Std.int(player.trueAge) % 3 == 0)
            {
                var factor = ServerSettings.DisplayScoreFactor;
                var textFromChildren = player.prestigeFromChildren > 0 ? ' From children ${player.prestigeFromChildren * factor}' : '';
                var textFromFollowers = player.prestigeFromFollowers > 0 ? ' From followers ${player.prestigeFromFollowers * factor}' : '';
                var message = 'You have gained ${player.prestigeFromEating * factor} prestige from YUMMY food!${textFromChildren}${textFromFollowers}';
                trace('New Age: $message');
                player.connection.sendGlobalMessage(message);
            }

            ScoreEntry.ProcessScoreEntry(player);
        }
    }

    private static function updateFoodAndDoHealing(player:GlobalPlayerInstance, timePassedInSeconds:Float)
    {
        //trace('food_store: ${connection.player.food_store}');

        var tmpFood = Math.ceil(player.food_store);
        var tmpExtraFood = Math.ceil(player.yum_bonus);
        var tmpFoodStoreMax = Math.ceil(player.food_store_max);
        var foodDecay = timePassedInSeconds * player.foodUsePerSecond; // depends on temperature
        var playerIsStarvingOrHasBadHeat = player.food_store < 0 || player.isSuperCold() || player.isSuperHot();
        
        if(player.age < ServerSettings.GrownUpAge && player.food_store > 0) foodDecay *= ServerSettings.IncreasedFoodNeedForChildren;

        var originalFoodDecay = foodDecay;
        var healing = timePassedInSeconds * 2.5 * ServerSettings.FoodUsePerSecond - foodDecay; // higher food need ==> less healing...
        

        // do damage if wound
        if(player.isWounded())
        {
            var bleedingDamage = timePassedInSeconds * player.heldObject.objectData.damage * ServerSettings.WoundDamageFactor;
            player.hits += bleedingDamage;
            foodDecay += 2 * bleedingDamage;
            //player.exhaustion += bleedingDamage;
        }

        // do damage while starving
        if(player.food_store < 0)
        {
            player.hits += foodDecay * ServerSettings.FoodStoreMaxReductionWhileStarvingToDeath / 2;
        }

        // take care of exhaustion
        if(player.exhaustion > -player.food_store_max && player.food_store > 0)
        {
            player.exhaustion -= healing * ServerSettings.ExhaustionHealing;

            foodDecay += originalFoodDecay * ServerSettings.ExhaustionHealing;

            if(player.exhaustion < -player.food_store_max) player.exhaustion = -player.food_store_max; 
        }
        
        // take damage if temperature is too hot or cold
        var damage:Float = 0;
        if(player.isSuperHot())  damage = player.heat > 0.98 ? 3 * foodDecay : foodDecay;
        else if(player.isSuperCold()) damage = player.heat < 0.02 ? 3 * foodDecay : foodDecay;
        
        damage /= 2;
        player.hits += damage;
        player.exhaustion += damage;

        // do Biome exhaustion
        //var tmpexhaustion = player.exhaustion;
        var biomeLoveFactor = player.biomeLoveFactor();
        if(biomeLoveFactor < 0) player.exhaustion -= foodDecay * biomeLoveFactor;
        if(biomeLoveFactor > 0 && player.exhaustion > -player.food_store_max) player.exhaustion -= foodDecay * biomeLoveFactor / 2;
        //trace('Exhaustion: $tmpexhaustion ==> ${player.exhaustion} pID: ${player.p_id} biomeLoveFactor: $biomeLoveFactor');

        // do healing but increase food use
        if(player.hits > 0 && playerIsStarvingOrHasBadHeat == false && player.isWounded() == false) 
        {
            player.hits -= healing / 2;

            foodDecay += originalFoodDecay;

            if(player.hits < 0) player.hits = 0; 

            if(player.woundedBy != 0 && player.hits < 1)
            {
                player.woundedBy = 0;
                if(player.connection != null) player.connection.send(ClientTag.HEALED, ['${player.p_id}']);
            }
        }

        // if starving to death and there is some health left, reduce food need and heath
        if(player.food_store < 0 && player.yum_multiplier > 0)
        {
            player.yum_multiplier -= foodDecay;
            foodDecay /= 2;
        }
        
        // do breast feeding
        var heldPlayer = player.heldPlayer;

        if(player.isHoldingChildInBreastFeedingAgeAndCanFeed())
        {
            if(ServerSettings.useOnePlayerMutex == false) heldPlayer.mutex.acquire(); // TODO can create a dead lock

            try{
                //trace('feeding:');

                if(heldPlayer.food_store < heldPlayer.food_store_max)
                {
                    var food = 5 * timePassedInSeconds * ServerSettings.FoodUsePerSecond; 

                    heldPlayer.food_store += food;
                    
                    foodDecay += food / 2;

                    //trace('feeding: $food foodDecay: $foodDecay');
                }
            }
            catch(ex)
            {
                trace('WARNING: ' + ex.details);
            }

            if(ServerSettings.useOnePlayerMutex == false) heldPlayer.mutex.release();
        }

        foodDecay *= player.isEveOrAdam() ? ServerSettings.EveFoodUseFactor : 1;

        if(player.yum_bonus > 0)
        {
            player.yum_bonus -= foodDecay;
        }
        else
        {
            player.food_store -= foodDecay;
        }

        player.food_store_max = player.calculateFoodStoreMax();

        var hasChanged = tmpFood != Math.ceil(player.food_store) || tmpExtraFood != Math.ceil(player.yum_bonus);
        hasChanged = hasChanged || tmpFoodStoreMax != Math.ceil(player.food_store_max);

        if(hasChanged)
        {
            player.sendFoodUpdate(false);
            if(player.connection != null) player.connection.send(FRAME, null, false);            
        }

        if(player.food_store_max < ServerSettings.DeathWithFoodStoreMax)
        {
            var reason = player.woundedBy == 0 ? 'reason_hunger': 'reason_killed_${player.woundedBy}';

            player.doDeath(reason);

            Connection.SendUpdateToAllClosePlayers(player, false);
        }
    }

    
    /*
        HX
        heat food_time indoor_bonus#

        Tells player about their current heat value, food drain time, and indoor bonus.

        Food drain time and indoor bonus are in seconds.

        Food drain time is total including bonus.
    */

    // Heat is the player's warmth between 0 and 1, where 0 is coldest, 1 is hottest, and 0.5 is ideal.
    private static function updateTemperature(player:GlobalPlayerInstance)
    {
        var temperature = calculateTemperature(player);

        var clothingInsulation = player.calculateClothingInsulation() / 10; // clothing insulation can be between 0 and 2 for now

        var clothingHeatProtection = player.calculateClothingHeatProtection() / 10; // (1-Insulation) clothing heat protection can be between 0 and 2 for now

        temperature += clothingInsulation;

        //if(temperature > 0.5) temperature -= (temperature - 0.5) * clothingHeatProtection; // the hotter the better the heat protection

        if(temperature > 0.5)
        {
            temperature -= clothingHeatProtection; 
            if(temperature < 0.5) temperature = 0.5;
        } 

        //if(temperature < 0) temperature = 0;
        //if(temperature > 1) temperature = 1;

        var insulationFactor = clothingInsulation / 2 + 0.88; // between 0.88 and 0.98

        var heatProtectionFactor = clothingHeatProtection / 2 + 0.88; // between 0.88 and 0.98

        var clothingFactor = temperature < 0.5 ? insulationFactor : heatProtectionFactor; 

        if(player.heat < 0.5 && player.heat < temperature) clothingFactor -= 0.1; // heating is positiv, so allow it more
        else if(player.heat > 0.5 && player.heat > temperature) clothingFactor -= 0.1; // cooling is positiv, so allow it more

        var closestHeatObj = AiHelper.GetClosestHeatObject(player);

        if(closestHeatObj != null)
        {
            var distance = AiHelper.CalculateDistanceToObject(player, closestHeatObj) + 1;

            var closestHeatTemperature = closestHeatObj.objectData.heatValue / (20 * distance);
            temperature += closestHeatTemperature;

            // use only half impact of close heat object if negative
            if(temperature < 0.5 && closestHeatTemperature < 0)
            {
                temperature -= closestHeatTemperature / 2;
                if(temperature > 0.5) temperature = 0.5;
            } 
            if(temperature > 0.5 && closestHeatTemperature > 0)
            {
                temperature -= closestHeatTemperature / 2;
                if(temperature < 0.5) temperature = 0.5;
            } 

            //trace('${closestHeatObj.description} Heat: ${closestHeatObj.objectData.heatValue} distance: $distance');
        }

        // consider held object heat
        var heldObjectData = player.heldObject.objectData;
        if(heldObjectData.heatValue != 0) temperature += heldObjectData.heatValue / 20;

        // add SeasonTemperatureImpact
        temperature += SeasonTemperatureImpact;

        var biomeLoveFactor = player.biomeLoveFactor();
        biomeLoveFactor /= 10;

        // TODO impact also how fast bad / good temperature effects player heat
        // balances temperature out if the biome is loved
        if(biomeLoveFactor > 0)
        {
            //trace('${player.p_id} biomeLoveFactor: $biomeLoveFactor');

            if(temperature < 0.5)
            {
                temperature += biomeLoveFactor;
                if(temperature > 0.5) temperature = 0.5;
            } 
            if(temperature > 0.5)
            {
                temperature -= biomeLoveFactor;
                if(temperature < 0.5) temperature = 0.5;
            } 
        }

        // If hold by other player, just use temperature from this instead
        if(player.heldByPlayer != null) temperature = player.heldByPlayer.heat;

        // TODO useTimePassed --> move to food update?
        player.heat = player.heat * clothingFactor + temperature * (1 - clothingFactor);

        if(player.heat > 1) player.heat = 1;
        if(player.heat < 0) player.heat = 0;
        
        var playerHeat = player.heat;

        var temperatureFoodFactor = playerHeat >= 0.5 ? playerHeat : 1 - playerHeat;

        var foodUsePerSecond = ServerSettings.FoodUsePerSecond * temperatureFoodFactor;

        var foodDrainTime = 1 / foodUsePerSecond;

        player.foodUsePerSecond = foodUsePerSecond;

        temperature = Math.round(temperature * 100) / 100;

        foodDrainTime = Math.round(foodDrainTime * 100) / 100;

        var message = '$playerHeat $foodDrainTime 0';

        if(player.connection != null) player.connection.send(HEAT_CHANGE, [message], false);

        //if(ServerSettings.DebugTemperature)
        //trace('Temperature update: playerHeat: $playerHeat temperature: $temperature clothingFactor: $clothingFactor foodDrainTime: $foodDrainTime foodUsePerSecond: $foodUsePerSecond clothingInsulation: $clothingInsulation clothingHeatProtection: $clothingHeatProtection');
    } 

    // Heat is the player's warmth between 0 and 1, where 0 is coldest, 1 is hottest, and 0.5 is ideal.
    private static function calculateTemperature(player:GlobalPlayerInstance) : Float
    {
        var maxBiomeDistance = 10; 
        // TODO consider close biome temperature influence
        var biome = WorldMap.worldGetBiomeId(player.tx, player.ty); 
        var originalBiomeTemperature = Biome.getBiomeTemperature(biome);
        var biomeTemperature = originalBiomeTemperature;

        // looke for close biomes that influence temperature
        if(biome == BiomeTag.GREEN || biome == BiomeTag.YELLOW || biome == BiomeTag.GREY)
        {         
            // direct x / y   
            for(ii in 1...maxBiomeDistance-1)
            {
                biomeTemperature = CalculateTemperatureAtPosition(player.tx + ii, player.ty, "+X", ii, maxBiomeDistance, biomeTemperature);
                if(biomeTemperature != originalBiomeTemperature) break; 
            }

            for(ii in 1...maxBiomeDistance-1)
            {
                biomeTemperature = CalculateTemperatureAtPosition(player.tx - ii, player.ty, "-X", ii, maxBiomeDistance, biomeTemperature);
                if(biomeTemperature != originalBiomeTemperature) break; 
            }

            for(ii in 1...maxBiomeDistance-1)
            {
                biomeTemperature = CalculateTemperatureAtPosition(player.tx, player.ty + ii, "+Y",  ii, maxBiomeDistance, biomeTemperature);
                if(biomeTemperature != originalBiomeTemperature) break; 
            }

            for(ii in 1...maxBiomeDistance-1)
            {
                biomeTemperature = CalculateTemperatureAtPosition(player.tx, player.ty - ii, "-Y", ii, maxBiomeDistance, biomeTemperature);
                if(biomeTemperature != originalBiomeTemperature) break; 
            }

            // diagonal x / y   
            for(ii in 1...maxBiomeDistance-1)
            {
                biomeTemperature = CalculateTemperatureAtPosition(player.tx + ii, player.ty + ii, "+X+Y", ii, maxBiomeDistance, biomeTemperature);
                if(biomeTemperature != originalBiomeTemperature) break; 
            }

            for(ii in 1...maxBiomeDistance-1)
            {
                biomeTemperature = CalculateTemperatureAtPosition(player.tx - ii, player.ty - ii, "-X-Y", ii, maxBiomeDistance, biomeTemperature);
                if(biomeTemperature != originalBiomeTemperature) break; 
            }

            for(ii in 1...maxBiomeDistance-1)
            {
                biomeTemperature = CalculateTemperatureAtPosition(player.tx - ii, player.ty + ii, "-X+Y", ii, maxBiomeDistance, biomeTemperature);
                if(biomeTemperature != originalBiomeTemperature) break; 
            }

            for(ii in 1...maxBiomeDistance-1)
            {
                biomeTemperature = CalculateTemperatureAtPosition(player.tx + ii, player.ty - ii, "+X-Y", ii, maxBiomeDistance, biomeTemperature);
                if(biomeTemperature != originalBiomeTemperature) break; 
            }
        }

        var colorTemperatureShift = getIdealTemperatureShiftForColor(player.getColor());
        
        // between -0.35 (black in snow) to 1.2 Ginger in dessert
        var temperature = biomeTemperature - colorTemperatureShift;  

        if(ServerSettings.DebugTemperature) trace('calculateTemperature: temperature: $temperature biomeTemperature: $biomeTemperature colorTemperatureShift: $colorTemperatureShift');

        return temperature;
    }

    private static function getIdealTemperatureShiftForColor(personColor:PersonColor) : Float
    {
        if(personColor == PersonColor.Black) return 0.35; // ideal temperature = 0.85
        if(personColor == PersonColor.Brown) return 0.2;
        if(personColor == PersonColor.White) return 0;
        if(personColor == PersonColor.Ginger) return -0.2; // ideal temperature = 0.3

        return 0; 
    }

    private static function CalculateTemperatureAtPosition(tx:Int, ty:Int, debugString:String, distance:Int, maxBiomeDistance:Int, originalTemperature:Float) : Float
    {
        var biome = WorldMap.worldGetBiomeId(tx, ty);
        var biomeTemperature = originalTemperature;

        if(biome == BiomeTag.DESERT || biome == BiomeTag.JUNGLE || biome == BiomeTag.SNOW)
        {
            var tmpBiomeTemperature = Biome.getBiomeTemperature(biome);
            biomeTemperature = (originalTemperature * distance + tmpBiomeTemperature * (maxBiomeDistance - distance)) / maxBiomeDistance;

            if(ServerSettings.DebugTemperature) trace('TEST BiomeTemp: $debugString distance: $distance biomeTemperature: $biomeTemperature tmpBiomeTemperature: $tmpBiomeTemperature');
        }

        return biomeTemperature;
    }

    public static function DoWorldMapTimeStuff()
    {
        // devide in X steps
        var timeParts = ServerSettings.WorldTimeParts; 
        var worldMap = Server.server.map;

        var partSizeY = Std.int(worldMap.height / timeParts);
        var startY = (worldMapTimeStep % timeParts) * partSizeY;
        var endY = startY + partSizeY;

        //trace('startY: $startY endY: $endY worldMap.height: ${worldMap.height}');

        if(worldMapTimeStep % timeParts == 0)
        {
            if(TimeTimeStepsSartedInTicks > 0) TimePassedToDoAllTimeSteps = TimeHelper.CalculateTimeSinceTicksInSec(TimeTimeStepsSartedInTicks);

            //trace('DOTIME: started: $TimeTimeStepsSartedInTicks passed: $TimePassedToDoAllTimeSteps');

            WinterDecayChance = TimePassedToDoAllTimeSteps * ServerSettings.WinterWildFoodDecayChance / (TimeToNextSeasonInYears * 60);
            SpringRegrowChance = TimePassedToDoAllTimeSteps * ServerSettings.SpringWildFoodRegrowChance / (TimeToNextSeasonInYears * 60);

            WinterDecayChance *= SeasonHardness;
            SpringRegrowChance *= SeasonHardness;
            
            TimeTimeStepsSartedInTicks = tick;

            //trace('DOTIME: winterDecayChance: $winterDecayChance springRegrowChance: $springRegrowChance');
        }

        worldMapTimeStep++;
        
        for (y in startY...endY)
        {
            for(x in 0...worldMap.width)
            {
                if(Season == Seasons.Spring)
                {
                    var hiddenObj = worldMap.getHiddenObjectId(x,y);
                    if(hiddenObj[0] != 0) RespawnOrDecayPlant(hiddenObj, x, y, true);  
                    
                    var originalObj = worldMap.getOriginalObjectId(x,y);
                    if(originalObj[0] != 0) RespawnOrDecayPlant(originalObj, x, y, false, ServerSettings.GrowBackOriginalPlantsFactor);    
                }

                var obj = worldMap.getObjectId(x,y);

                if(obj[0] == 0) continue;      
                
                RespawnOrDecayPlant(obj, x, y);

                var helper = worldMap.getObjectHelper(x,y,true); 

                if(helper != null)
                {              
                    if(helper.timeToChange == 0) // maybe timeToChange was forgotten to be set
                    {
                        var timeTransition = TransitionImporter.GetTransition(-1, obj[0], false, false);

                        if(timeTransition == null) continue;

                        trace('WARNING: found helper without time transition: ${helper.description}');

                        helper.timeToChange = ObjectHelper.CalculateTimeToChange(timeTransition);
                    }

                    // clear up not needed ObjectHelpers to save space
                    if(worldMap.deleteObjectHelperIfUseless(helper)) continue; // uses worlmap mutex

                    TimeHelper.doTimeTransition(helper);

                    continue;
                }

                var timeTransition = TransitionImporter.GetTransition(-1, obj[0], false, false);
                if(timeTransition == null) continue;

                helper = worldMap.getObjectHelper(x,y); 
                helper.timeToChange = ObjectHelper.CalculateTimeToChange(timeTransition);

                worldMap.setObjectHelper(x,y,helper);

                //trace('TIME: ${helper.objectData.description} neededTime: ${timeToChange}');  
                
                //var testObj = getObjectId(helper.tx, helper.ty);

                //trace('testObj: $testObj obj: $obj ${helper.tx},${helper.ty} i:$i index:${index(helper.tx, helper.ty)}');
            }
        }
    }  

    private static function RespawnOrDecayPlant(objIDs:Array<Int>, x:Int, y:Int, hidden:Bool = false, growFactor:Float = 1)
    {
        var objID = objIDs[0];
        var objData = ObjectData.getObjectData(objID);

        if(Season == Seasons.Winter && hidden == false && objData.winterDecayFactor > 0)
        {
            // reduce uses if it is for example a berry bush
            if(objData.numUses > 1)
            {                
                var objHelper = WorldMap.world.getObjectHelper(x,y);
                
                if(WinterDecayChance * objData.winterDecayFactor * objHelper.numberOfUses < WorldMap.calculateRandomFloat()) return;
                
                WorldMap.world.mutex.acquire(); // TODO try catch // TODO object helper may have changed

                objHelper.numberOfUses -= 1;                
                objHelper.TransformToDummy();
                WorldMap.world.setObjectHelper(x,y, objHelper);
                Connection.SendMapUpdateToAllClosePlayers(x,y, [objHelper.id]);

                WorldMap.world.mutex.release();

                return;
            }

            if(WinterDecayChance * objData.winterDecayFactor < WorldMap.calculateRandomFloat()) return;

            var random = WorldMap.calculateRandomFloat();            

            WorldMap.world.mutex.acquire(); // TODO try catch

            WorldMap.world.setObjectId(x, y, [0]);
            if(objData.springRegrowFactor > random) WorldMap.world.setHiddenObjectId(x, y, objIDs); // TODO hide also object helper for advanced objects???

            Connection.SendMapUpdateToAllClosePlayers(x,y,[0]);

            WorldMap.world.currentObjectsCount[objID]--;

            WorldMap.world.mutex.release(); // TODO try catch

            if(ServerSettings.DebugSeason)
            {
                var mod = WorldMap.world.currentObjectsCount[objID] < 1000 ? 100 : 1000;
                mod = WorldMap.world.currentObjectsCount[objID] < 100 ? 10 : mod;
                mod = WorldMap.world.currentObjectsCount[objID] < 10 ? 1 : mod;  
                if(WorldMap.world.currentObjectsCount[objID] % mod == 0) trace('SEASON DECAY: ${objData.description} ${WorldMap.world.currentObjectsCount[objID]} original: ${WorldMap.world.originalObjectsCount[objID]}');
            }
        }
        else if(Season == Seasons.Spring && objData.springRegrowFactor > 0)
        {
            // TODO regrow also from originalObjects?
            
            // increase uses if it is for example a berry bush
            if(objData.numUses > 1 || objData.undoLastUseObject != 0)
            {
                var objHelper = WorldMap.world.getObjectHelper(x,y);
                if(objHelper.numberOfUses >= objData.numUses && objData.undoLastUseObject == 0) return;

                var factor = objData.numUses - objHelper.numberOfUses;
                if(factor < 1) factor = 1;

                if(SpringRegrowChance * objData.springRegrowFactor * factor < WorldMap.calculateRandomFloat()) return;

                WorldMap.world.mutex.acquire(); // TODO try catch

                objHelper.numberOfUses += 1;

                objHelper.TransformToDummy();
                WorldMap.world.setObjectHelper(x,y, objHelper);

                WorldMap.world.mutex.release(); // TODO try catch

                Connection.SendMapUpdateToAllClosePlayers(x,y, [objHelper.id]);

                return;
            }

            var factor = hidden ? 2 : 0.2;
            factor *= growFactor;

            if(SpringRegrowChance * objData.springRegrowFactor * factor < WorldMap.calculateRandomFloat()) return;

            var spawnAs = objData.countsOrGrowsAs > 0 ? objData.countsOrGrowsAs : objID;

            WorldMap.world.mutex.acquire(); // TODO try catch

            var done = SpawnObject(x,y,spawnAs);

            if(hidden && done) WorldMap.world.setHiddenObjectId(x, y, [0]); // What was hidden comes back

            WorldMap.world.mutex.release(); // TODO try catch

            if(ServerSettings.DebugSeason)
            {
                var mod = WorldMap.world.currentObjectsCount[spawnAs] < 1000 ? 100 : 1000;
                mod = WorldMap.world.currentObjectsCount[spawnAs] < 100 ? 10 : mod;
                mod = WorldMap.world.currentObjectsCount[spawnAs] < 10 ? 1 : mod;  
                if(WorldMap.world.currentObjectsCount[spawnAs] % mod == 0) trace('SEASON REGROW: ${objData.description} ${WorldMap.world.currentObjectsCount[spawnAs]} original: ${WorldMap.world.originalObjectsCount[spawnAs]}');
            }
        }
    }

    public static function RespawnObjects()
    {
        var timeParts = ServerSettings.WorldTimeParts * 10; 
        var worldMap = Server.server.map;
        var partSizeY = Std.int(worldMap.height / timeParts);
        var startY = (worldMapTimeStep % timeParts) * partSizeY;
        var endY = startY + partSizeY;

        //trace('startY: $startY endY: $endY worldMap.height: ${worldMap.height}');

        for (y in startY...endY)
        {
            for(x in 0...worldMap.width)
            {
                var objID = worldMap.getOriginalObjectId(x,y)[0];
                
                if(objID == 0) continue;

                if(ServerSettings.CanObjectRespawn(objID) == false) continue;

                if(ServerSettings.ObjRespawnChance < worldMap.randomFloat()) continue;

                WorldMap.world.mutex.acquire(); // TODO try catch

                SpawnObject(x, y, objID);

                WorldMap.world.mutex.release(); // TODO try catch

                //trace('respawn object: ${objData.description} $obj');
            }
        }    
    }

    public static function SpawnObject(x:Int, y:Int, objID:Int, dist:Int = 6, tries:Int = 3) : Bool
    {
        var worldMap = WorldMap.world;

        if(worldMap.currentObjectsCount[objID] >= worldMap.originalObjectsCount[objID]) return false;

        for(ii in 0...tries)
        {
            var tmpX = worldMap.randomInt(2*dist) - dist + x;
            var tmpY = worldMap.randomInt(2*dist) - dist + y;

            if(worldMap.getObjectId(tmpX, tmpY)[0] != 0) continue;

            if(worldMap.getObjectId(tmpX, tmpY-1)[0] != 0) continue; // make sure that obj does not spawn above one tile of existing obj

            var biomeId = worldMap.getBiomeId(tmpX,tmpY);
            var objData = ObjectData.getObjectData(objID);

            if(objData.isSpawningIn(biomeId) == false) continue;

            worldMap.setObjectId(tmpX, tmpY, [objID]);

            worldMap.currentObjectsCount[objID]++;

            Connection.SendMapUpdateToAllClosePlayers(tmpX, tmpY, [objID]);

            return true;
        }

        return false;
    } 

    public static function DecayObjects()
    {
        // TODO decay stuff in containers
        // TODO decay stuff with number of uses > 1
        // TODO create custom decay transitions
        // TODO add decay object so that decay is visible

        var timeParts = ServerSettings.WorldTimeParts * 10; 
        var worldMap = Server.server.map;
        var partSizeY = Std.int(worldMap.height / timeParts);
        var startY = (worldMapTimeStep % timeParts) * partSizeY;
        var endY = startY + partSizeY;

        //trace('startY: $startY endY: $endY worldMap.height: ${worldMap.height}');

        for (y in startY...endY)
        {
            for(x in 0...worldMap.width)
            {
                var obj = worldMap.getObjectId(x,y)[0];
                
                if(obj == 0) continue;

                if(ServerSettings.CanObjectRespawn(obj) == false) continue;

                if(worldMap.currentObjectsCount[obj] <= worldMap.originalObjectsCount[obj]) continue; // dont decay natural stuff if there are too few

                var objectHelper = worldMap.getObjectHelper(x,y, true);

                if(objectHelper != null && objectHelper.containedObjects.length > 0) continue; // TODO change

                if(objectHelper != null && objectHelper.timeToChange > 0) continue; // dont decay object if there is a time transition

                var objData = ObjectData.getObjectData(obj);
                
                if(objData.decayFactor <= 0) continue;       

                var decayChance = ServerSettings.ObjDecayChance * objData.decayFactor;

                if(objData.foodValue > 0) decayChance *= ServerSettings.ObjDecayFactorForFood;

                if(worldMap.getFloorId(x,y) != 0) decayChance *= ServerSettings.ObjDecayFactorOnFloor;

                if(worldMap.randomFloat() > decayChance) continue;

                //if(objData.isSpawningIn(biomeId) == false) continue;

                worldMap.setObjectId(x,y, [0]);
                //worldMap.setObjectHelperNull(x, y);

                worldMap.currentObjectsCount[obj]--;

                Connection.SendMapUpdateToAllClosePlayers(x, y, [0]);

                //trace('decay object: ${objData.description} $obj');
            }
        }    
    }

    public static function RespawnObj()
    {
    }

    public static function doTimeTransition(helper:ObjectHelper)
    {
        if(helper.isTimeToChangeReached() == false) return; 
        //trace('TIME: ${helper.objectData.description} passedTime: $passedTime neededTime: ${timeToChange}');  

        // TODO test time transition for maxUseTaget like Goose Pond:
        // -1 + 142 = 0 + 142
        // -1 + 142 = 0 + 141

        WorldMap.world.mutex.acquire();
        
        // just to be sure, that no other thread changed object meanwhile 
        
        if(helper != WorldMap.world.getObjectHelper(helper.tx, helper.ty))
        {
            WorldMap.world.mutex.release();

            trace("TIME: some one changed helper meanwhile");

            return;
        } 

        var sendUpdate = false;

        Macro.exception(sendUpdate = doTimeTransitionHelper(helper));
       
        WorldMap.world.mutex.release();

        if(sendUpdate == false) return;

        var newTileObject = helper.toArray();

        Connection.SendMapUpdateToAllClosePlayers(helper.tx, helper.ty, newTileObject);
    } 

    public static function doTimeTransitionHelper(helper:ObjectHelper) : Bool
    {
        var tx = helper.tx;
        var ty = helper.ty;

        var tileObject = Server.server.map.getObjectId(tx, ty);

        //trace('Time: tileObject: $tileObject');

        var transition = TransitionImporter.GetTransition(-1, tileObject[0], false, false);

        if(transition == null)
        {
            helper.timeToChange = 0;
            WorldMap.world.setObjectHelperNull(tx,ty);

            trace('WARNING: Time: no transtion found! Maybe object was moved? tile: $tileObject helper: ${helper.id} ${helper.description}');
            return false;
        }

        var newObjectData = ObjectData.getObjectData(transition.newTargetID);

        // for example if a grave with objects decays
        if(helper.containedObjects.length > newObjectData.slotSize)
        {
            // check in another 20 sec
            helper.timeToChange += 20;

            var containedObject = helper.containedObjects.pop();

            // For each Sharp Stone a grave needs much longer to decay / This can be used to let cursed graves exist much longer
            if(containedObject.id == 34)
            {
                helper.timeToChange += ServerSettings.CursedGraveTime * 60 * 60;    
                ScoreEntry.CreateScoreEntryForCursedGrave(helper);
            }

            WorldMap.PlaceObject(tx, ty, containedObject);

            trace('time: placed object ${containedObject.description} from ${helper.description}');

            //trace('time: could not decay newTarget cannot store contained objects! ${helper.description}');

            return false;
        }

        if(transition.move > 0)
        {
            doAnimalMovement(helper, transition);
            
            return false;
        }

        ScoreEntry.CreateScoreEntryIfGrave(helper);
        
        if(helper.isLastUse()) transition = TransitionImporter.GetTransition(-1, helper.id, false, true);

        helper.id = transition.newTargetID;
        helper.timeToChange = ObjectHelper.CalculateTimeToChangeForObj(helper);
        helper.creationTimeInTicks = TimeHelper.tick;

        TransitionHelper.DoChangeNumberOfUsesOnTarget(helper, transition, false);

        WorldMap.world.setObjectHelper(tx, ty, helper);

        return true;
    }

    /*
        MX
        x y new_floor_id new_id p_id old_x old_y speed

        Optionally, a line can contain old_x, old_y, and speed.
        This indicates that the object came from the old coordinates and is moving
        with a given speed.
    */

    private static function doAnimalMovement(helper:ObjectHelper, timeTransition:TransitionData) : Bool
    {
        // TODO chasing
        // TODO fleeing
        // TODO Offspring only if with child

        var moveDist = timeTransition.move;

        if(moveDist <= 0) return false;

        moveDist += 1; // movement distance is plus 4 in original code if walking over objects
        helper.objectData.moves = moveDist; // TODO better set in settings

        if(helper.hits > 0) helper.hits -= 0.02; // reduce hits the animal got
        if(helper.hits < 0) helper.hits = 0;

        var worldmap = Server.server.map;

        var fromTx = helper.tx;
        var fromTy = helper.ty;

        for (i in 0...20)
        {
            var toTx = helper.tx - moveDist + worldmap.randomInt(moveDist * 2);
            var toTy = helper.ty - moveDist + worldmap.randomInt(moveDist * 2);
            
            var target = worldmap.getObjectHelper(toTx, toTy);

            if(target.id != 0) continue;

            if(target.blocksWalking()) continue;
            // dont move move on top of other moving stuff
            if(target.timeToChange != 0) continue;  
            if(target.groundObject != null) continue;
            // make sure that target is not the old tile
            if(toTx == helper.tx && toTy == helper.ty) continue;
            
            var targetBiome = worldmap.getBiomeId(toTx,toTy);
            
            if(WorldMap.isBiomeBlocking(toTx, toTy)) continue; 

            var objectData = helper.objectData.dummyParent == null ? helper.objectData : helper.objectData.dummyParent;

            var isPreferredBiome = objectData.isSpawningIn(targetBiome);

            //if(helper.objectData.dummyParent != null) trace('Animal Move: ${objectData.description} $isPreferredBiome');

            // lower the chances even more if on river
            //var isHardbiome = targetBiome == BiomeTag.RIVER || (targetBiome == BiomeTag.GREY) || (targetBiome == BiomeTag.SNOW) || (targetBiome == BiomeTag.DESERT);
            var isNotHardbiome =  isPreferredBiome || targetBiome == BiomeTag.GREEN || targetBiome == BiomeTag.YELLOW;

            var chancePreferredBiome = isNotHardbiome ? ServerSettings.chancePreferredBiome : (ServerSettings.chancePreferredBiome + 4) / 5;

            //trace('chance: $chancePreferredBiome isNotHardbiome: $isNotHardbiome biome: $targetBiome');

            // skip with chancePreferredBiome if this biome is not preferred
            if(isPreferredBiome == false && i < Math.round(chancePreferredBiome * 10) &&  worldmap.randomFloat() <= chancePreferredBiome) continue;

            // limit movement if blocked
            target = calculateNonBlockedTarget(fromTx, fromTy, target);

            if(target == null) continue; // movement was fully bocked, search another target

            toTx = target.tx;
            toTy = target.ty;

            // 2710 + -1 = 767 + 769 // Wild Horse with Lasso + TIME  -->  Lasso# tool + Wild Horse
            var transition = TransitionImporter.GetTransition(helper.parentId, -1, false, false);

            if(transition != null)
            {            
                var tmpGroundObject = helper.groundObject;
                helper.groundObject = new ObjectHelper(null, transition.newActorID);
                helper.groundObject.groundObject = tmpGroundObject;

                //trace('animal movement: found -1 transition: ${helper.description} --> ${helper.groundObject.description}');
            } 

            if(helper.isLastUse()) timeTransition = TransitionImporter.GetTransition(-1, helper.id, false, true);

            helper.id = timeTransition.newTargetID;

            TransitionHelper.DoChangeNumberOfUsesOnTarget(helper, timeTransition, false);

            helper.tx = toTx;
            helper.ty = toTy;
            var damage = DoAnimalDamage(fromTx, fromTy, helper);
    
            // save what was on the ground, so that we can move on this tile and later restore it
            var oldTileObject = helper.groundObject == null ? [0]: helper.groundObject.toArray();
            var newTileObject = helper.toArray();

            //var des = helper.groundObject == null ? "NONE": helper.groundObject.description();
            //trace('MOVE: oldTile: $oldTileObject $des newTile: $newTileObject ${helper.description()}');

            // TODO only change after movement is finished
            if(damage <= 0) helper.timeToChange = ObjectHelper.CalculateTimeToChange(timeTransition);
            helper.creationTimeInTicks = TimeHelper.tick;

            worldmap.setObjectHelper(fromTx, fromTy, helper.groundObject);

            var tmpGroundObject = helper.groundObject;
            helper.groundObject = target;
            worldmap.setObjectHelper(toTx, toTy, helper);

            var chanceForOffspring = isPreferredBiome ? ServerSettings.ChanceForOffspring : ServerSettings.ChanceForOffspring * Math.pow((1 - chancePreferredBiome), 2);
            var chanceForAnimalDying = isPreferredBiome ? ServerSettings.ChanceForOffspring / 2: ServerSettings.ChanceForAnimalDying;

            // give extra birth chance bonus if population is very low
            if(worldmap.currentObjectsCount[newTileObject[0]] < worldmap.originalObjectsCount[newTileObject[0]] / 2) chanceForOffspring *=5;

            if(worldmap.currentObjectsCount[newTileObject[0]] < worldmap.originalObjectsCount[newTileObject[0]] * ServerSettings.MaxOffspringFactor && worldmap.randomFloat() <= chanceForOffspring)
            {
                worldmap.currentObjectsCount[newTileObject[0]] += 1;

                //if(chanceForOffspring < worldmap.chanceForOffspring) trace('NEW: $newTileObject ${helper.description()}: ${worldmap.currentPopulation[newTileObject[0]]} ${worldmap.initialPopulation[newTileObject[0]]} chance: $chanceForOffspring biome: $targetBiome');

                oldTileObject = newTileObject;
                
                var newAnimal = ObjectHelper.readObjectHelper(null, newTileObject);
                newAnimal.timeToChange = ObjectHelper.CalculateTimeToChange(timeTransition);
                newAnimal.groundObject = tmpGroundObject;
                worldmap.setObjectHelper(fromTx, fromTy, newAnimal);
            }
            else if(worldmap.currentObjectsCount[newTileObject[0]] > worldmap.originalObjectsCount[newTileObject[0]] * ServerSettings.MaxOffspringFactor && worldmap.randomFloat() <= chanceForAnimalDying)
            {
                //trace('Animal DEAD: $newTileObject ${helper.description}: Count: ${worldmap.currentObjectsCount[newTileObject[0]]} Original Count: ${worldmap.originalObjectsCount[newTileObject[0]]} chance: $chanceForAnimalDying biome: $targetBiome');

                helper.id = 0;
                newTileObject = [0];
                worldmap.setObjectHelper(toTx, toTy, helper);
            }       

            var speed = ServerSettings.InitialPlayerMoveSpeed * objectData.speedMult;
            Connection.SendAnimalMoveUpdateToAllClosePlayers(fromTx, fromTy, toTx, toTy, oldTileObject, newTileObject, speed);

            return true;
        }

        helper.creationTimeInTicks = TimeHelper.tick;

        return false;
    }    

    public static function TryAnimaEscape(attacker:GlobalPlayerInstance, target:ObjectHelper) : Bool
    {
        var weapon = attacker.heldObject;
        var animalEscapeFactor = weapon.objectData.animalEscapeFactor - target.hits * 0.2;
        var random = WorldMap.calculateRandomFloat();

        trace('TryAnimaEscape: ${target.hits} $random > $animalEscapeFactor');
        target.hits += 1;

        if(random > animalEscapeFactor) return false;

        // TODO set hits to incease chance

        target.timeToChange /= 5;    
        var tmpTimeToChange = target.timeToChange;
        doTimeTransition(target);

        var escaped = tmpTimeToChange != target.timeToChange;
        trace('TryAnimaEscape: $escaped');
        if(escaped == false) return false;

        if(weapon.id == 152) // Bow and Arrow
        {
            weapon.id = 749; // 151 Bow // 749 Bloody Yew Bow
            attacker.setHeldObject(weapon);
            attacker.setHeldObjectOriginNotValid(); // no animation 
            weapon.timeToChange = 2; 

            WorldMap.PlaceObject(target.tx, target.ty, new ObjectHelper(attacker, 798), true); // Place Arrow Wound
        }

        return true;
    }

    public static function MakeAnimalsRunAway(player:GlobalPlayerInstance, searchDistance:Int = 1)
    {
        //AiHelper.GetClosestObject
        var world = WorldMap.world;
        var baseX = player.tx;
        var baseY = player.ty;

        for(ty in baseY - searchDistance...baseY + searchDistance)
        {
            for(tx in baseX - searchDistance...baseX + searchDistance)
            {
                var obj = world.getObjectHelper(tx, ty, true);
                if(obj == null) continue;
                if(obj.objectData.moves == 0) continue;

                var tmpTimeToChange = obj.timeToChange;
                obj.timeToChange /= 5;    
                doTimeTransition(obj);

                //trace('RUN: $tmpTimeToChange --> ${obj.timeToChange} ' + obj.description);
                //obj.timeToChange = tmpTimeToChange;
            }
        }
    }

    private static function calculateNonBlockedTarget(fromX:Int, fromY:Int, toTarget:ObjectHelper) : ObjectHelper
    {
        var tx = toTarget.tx;
        var ty = toTarget.ty;
        var tmpX = fromX;
        var tmpY = fromY;
        var tmpTarget = null;

        for(ii in 0...10)
        {                
            if(tmpX == tx && tmpY == ty) break;

            if(tx > tmpX)  tmpX += 1;
            else if(tx < tmpX)  tmpX -= 1;

            if(ty > tmpY)  tmpY += 1;
            else if(ty < tmpY)  tmpY -= 1;

            //trace('movement: $tmpX,$tmpY');

            var movementTileObj = WorldMap.worldGetObjectHelper(tmpX , tmpY); 
            //var movementBiome = WorldMap.worldGetBiomeId(tmpX , tmpY);  

            //var cannotMoveInBiome = movementBiome == BiomeTag.OCEAN ||  movementBiome == BiomeTag.SNOWINGREY;

            var isBiomeBlocking = WorldMap.isBiomeBlocking(tmpX, tmpY);

            if(isBiomeBlocking && ServerSettings.ChanceThatAnimalsCanPassBlockingBiome > 0) isBiomeBlocking = WorldMap.calculateRandomFloat() > ServerSettings.ChanceThatAnimalsCanPassBlockingBiome; 

            // TODO better patch in the objects, i dont see any reason why a rabit or a tree should block movement
            if(isBiomeBlocking || (movementTileObj.blocksWalking() 
                    && movementTileObj.description.indexOf("Tarry Spot") == -1
                    && movementTileObj.description.indexOf("Tree") == -1 && movementTileObj.description.indexOf("Rabbit") == -1  
                    && movementTileObj.description.indexOf("Iron") == -1 && movementTileObj.description.indexOf("Spring") == -1
                    && movementTileObj.description.indexOf("Sugarcane") == -1 && movementTileObj.description.indexOf("Pond") == -1
                    && movementTileObj.description.indexOf("Palm") == -1  && movementTileObj.description.indexOf("Plant") == -1))
            {
                //trace('movement blocked ${movementTile.description()} ${movementBiome}');
                break;
            }
            

            // TODO allow move on non empty ground
            if(movementTileObj.id == 0) tmpTarget = movementTileObj;
        }
   
        return tmpTarget;
    }

    private static function DoAnimalDamage(fromX:Int, fromY:Int, animal:ObjectHelper) : Float
    {
        var objData = animal.objectData;
        
        if(objData.deadlyDistance <= 0) return 0;
        if(objData.damage <= 0) return 0;

        //trace('${objData.description} deadlyDistance: ${objData.deadlyDistance} damage: ${objData.damage}');
        var damage = 0.0;
        var tx = animal.tx;
        var ty = animal.ty;
        var tmpX = fromX;
        var tmpY = fromY;

        for(ii in 0...10)
        {                
            if(tmpX == tx && tmpY == ty) break;

            if(tx > tmpX)  tmpX += 1;
            else if(tx < tmpX)  tmpX -= 1;

            if(ty > tmpY)  tmpY += 1;
            else if(ty < tmpY)  tmpY -= 1;

            for(p in GlobalPlayerInstance.AllPlayers)
            {
                if(p.deleted) continue;
                if(p.isCloseUseExact(tmpX, tmpY, objData.deadlyDistance) == false) continue;

                damage += p.doDamage(animal);
                return damage;
            }
        }

        return damage;
    }

    // Called before time. Do tests here!
    private static function DoTest()
    {
        return; // remove if testting
        var trans = TransitionImporter.GetTransition(418, 0, false, false);
        trace('TRANS4: $trans false, false');
        var trans = TransitionImporter.GetTransition(418, 0, true, false);
        trace('TRANS4: $trans true, false');
        //var trans = TransitionImporter.GetTransition(418, 0, true, false);
        //trace('TRANS: $trans');
    }

    //static var personIndex = 0;
    //static var colorIndex = 0;
    private static function DoTimeTestStuff()
    {
        if(tick % 200 == 0) 
            {
                if(Connection.getConnections().length > 0)
                {
                    /*
                    var c = Connection.getConnections()[0];
                    //FW follower_id leader_id leader_color_index
                    c.send(ClientTag.FOLLOWING, ['${c.player.p_id} 2 $colorIndex']);
                    //p_id emot_index
                    c.send(ClientTag.PLAYER_EMOT, ['${c.player.p_id} $colorIndex']);
                    //c.send(ClientTag.PLAYER_SAYS, ['0 0 $colorIndex']);
                    //c.player.say('color $colorIndex');
                    c.send(ClientTag.LOCATION_SAYS, ['100 0 /LEADER']);
    
                    trace('FOLLOW '+ '${c.player.p_id} 2 $colorIndex');
    
                    colorIndex++;
                    
                    */
                    /*
                    c.player.po_id = obj.id;
                    
                    Connection.SendUpdateToAllClosePlayers(c.player);
                    if(tick % 200 == 0) c.send(ClientTag.DYING, ['${c.player.p_id}']);
                    else c.send(ClientTag.HEALED, ['${c.player.p_id}']);
    
                    c.sendGlobalMessage('Id ${obj.parentId} P${obj.person} ${obj.description}');
    
                    personIndex++;
                    //  418 + 0 = 427 + 1363 / @ Deadly Wolf + Empty  -->  Attacking Wolf + Bite Wound 
                    */            
                }
            }
    }
}