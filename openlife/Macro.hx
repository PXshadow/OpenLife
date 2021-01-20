package openlife;

import haxe.macro.Expr;

macro function exception(expr:Expr)
{
    return macro if (openlife.settings.ServerSetting.debug)
    {
        $expr
    }else{
        try {
            $expr
        }catch(e) {
            trace(e);
        }
    }
}