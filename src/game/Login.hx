package states.game;

import ui.Button;
import openfl.display.Sprite;
import ui.InputText;

//login shown up in game
class Login extends Sprite
{
    var emailInput:InputText;
    var passwordInput:InputText;
    var enter:Button = new Button();
    public function new()
    {
        super();
        emailInput = new InputText();
        passwordInput = new InputText();
        passwordInput.displayAsPassword = true;
        addChild(emailInput);
        addChild(passwordInput);

        enter.text = "Enter";
        enter.Click = function(_)
        {
            
        }
        addChild(enter);
    }
}