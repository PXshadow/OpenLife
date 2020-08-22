import openlife.resources.ObjectBake;
import openlife.engine.Engine;
import openlife.data.transition.*;
class Transition
{
    public function new()
    {
        var vector = ObjectBake.objectList();
        var importer = new TransitionImporter();
        importer.importCategories();
        importer.importTransitions();
    }
}