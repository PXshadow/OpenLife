import openlife.data.object.ObjectData;
import openlife.resources.ObjectBake;
import openlife.engine.Engine;
import openlife.data.transition.*;
class Transition
{
    public function new()
    {
        var vector = ObjectBake.objectList();
        var importer = new TransitionImporter();
        var deadly:Int = 0;
        for (transition in importer.transitions)
        {
            if (transition.isAttack())
            {
                Sys.println(new ObjectData(transition.actorID,true).description);
                deadly++;
                for (category in importer.categories)
                {
                    if (category.parentID == transition.actorID)
                    {
                        for (id in category.ids)
                        {
                            Sys.println(new ObjectData(id,true).description);
                            deadly++;
                        }
                    }
                }
            }
        }
        trace('deadly 0: $deadly');
        deadly = 0;
        for (id in vector)
        {
            var obj = new ObjectData(id);
            if (obj.deadlyDistance > 0)
            {
                Sys.println('id ' + obj.id + " " + obj.description);
                deadly++;
            }
        }
        trace('deadly 1: $deadly');
    }
}