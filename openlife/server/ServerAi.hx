package openlife.server;

import openlife.auto.Ai;

class ServerAi extends Ai
{
    public var me:GlobalPlayerInstance;

    public function new(player:GlobalPlayerInstance)
    {
        super(player, player);

        me = player;
    }   
}