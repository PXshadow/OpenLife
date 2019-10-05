package console;
//artifical intelligence, runs based on the logic detirmined by previous training data
class NeuralNetwork extends Network
{
    var inputs:Array<Float> = [];
    var outputs:Array<Float> = [];
    
    public function new()
    {
        super();
    }
    private function sigmoid(x:Float):Float
    {
        return 1 / (1 + Math.exp(-x));
    }
}