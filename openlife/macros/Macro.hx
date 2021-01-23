package openlife.macros;

import haxe.macro.Expr;

class Macro {
    public static macro function exception(expr:Expr)
    {
        return macro if (openlife.settings.ServerSettings.debug)
        {
            $expr;
        }else{
            try {
                $expr;
            }
            catch(e)
            {
                trace('WARNING: ' + e + '\n' + e.details() );
            }
        }
    }
}