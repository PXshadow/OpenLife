import openlife.engine.Utility;

class Test
{
    public static function main()
    {
        openlife.engine.Engine.dir = Utility.dir();
        Sys.println("Begin testing");
        new Transition();
    }
}