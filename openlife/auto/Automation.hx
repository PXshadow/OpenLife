package openlife.auto;

import openlife.engine.Program;


/**
 * nick name auto, will be a powerful class that uses program, and transition data to do automatic tasks.
 */
class Automation
{
    var program:Program;
    public function new(program:Program)
    {
        this.program = program;
    }
}
typedef Auto = Automation; 