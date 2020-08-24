package openlife.data.transition;

import openlife.data.object.ObjectData;
import haxe.ds.Vector;

class Recipe
{
    public var id:Int = 0; //c (value)
    public var a:Recipe = null; //actor
    public var b:Recipe = null; //target
    public function new(id:Int)
    {
        this.id = id;
    }
    public function generate(transitions:Array<TransitionData>,categories:Array<Category>,count:Int=0)
    {
        if (new ObjectData(id).isNatural()) return;
        /*for (category in categories)
        {
            if (category.parentID == id)
            {
                trace("category " + category.ids);
            }
        }*/
        var trans:Array<TransitionData> = [];
        for (transition in transitions)
        {
            if (transition.newActorID != id && transition.newTargetID != id) continue;
            if (transition.targetID == id || transition.actorID == id) continue;
            trans.push(transition);
        }
        if (trans.length == 0) return;
        trans.sort(function(a:TransitionData,b:TransitionData)
        {
            if (a.newActorID == 0 || b.newTargetID == 0) return a.newActorID - b.newTargetID;
            if (a.targetID == -1 || b.targetID == -1) return b.targetID - a.targetID;
            return a.targetID - b.targetID;
        });
        if (id == 57)
        {
            trace("MILKWEED:");
            for (tran in trans) trace(tran);
        }
        var tran = trans.shift();
        Sys.println(tran);
        if (++count > 10) throw "overflow count to much generation";
        trace("id " + tran.actorID + " " + tran.targetID);
        if (tran.actorID > 0)
        {
            a = new Recipe(tran.actorID);
            a.generate(transitions,categories,count);
        }
        if (tran.targetID > 0)
        {
            b = new Recipe(tran.targetID);
            b.generate(transitions,categories,count);
        }
    }
    public function depth():Int
    {
        var av = a == null ? 0 : a.depth();
        var bv = b == null ? 0 : b.depth();
        return (id > 0 ? 1 : 0) + (av > bv ? av : bv);
    }
}