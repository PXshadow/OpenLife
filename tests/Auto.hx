package;
import openlife.auto.BotType;
import openlife.engine.Engine;
import openlife.auto.actions.Take;
import openlife.auto.roles.*;
import openlife.auto.Action;
import openlife.auto.Role;
function main()
{
    var bot = new BotType(null);
    var role = new BerryEater();
    new Take().step(bot);
}