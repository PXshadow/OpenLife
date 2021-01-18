package openlife.server;

import openlife.auto.Ai;

class ServerAi extends Ai
{
    public var me:GlobalPlayerInstance;

    public function new(player:GlobalPlayerInstance)
    {
        super(me, me);

        me = player;
    }   
}