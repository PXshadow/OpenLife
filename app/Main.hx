package;

import openlife.engine.Engine;

class Main
{
    public static function main()
    {
        Sys.println("Starting OpenLife App Client"#if debug + " in debug mode" #end);
        new App();
    }
}