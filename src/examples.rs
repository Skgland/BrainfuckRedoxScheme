pub const CAT:&str =",[.,]";

pub const CELL_SIZE:&str = "
Calculate the value 256 and test if it's zero
If the interpreter errors on overflow this is where it'll happen
++++++++[>++++++++<-]>[<++++>-]
+<[>-<
    Not zero so multiply by 256 again to get 65536
    [>++++<-]>[<++++++++>-]<[>++++++++<-]
    +>[>
        # Print '32'
        ++++++++++[>+++++<-]>+.-.[-]<
    <[-]<->] <[>>
        # Print '16'
        +++++++[>+++++++<-]>.+++++.[-]<
<<-]] >[>
    # Print '8'
    ++++++++[>+++++++<-]>.[-]<
<-]<
# Print  bit cells\n
+++++++++++[>+++>+++++++++>+++++++++>+<<<<-]>-.>-.+++++++.+++++++++++.<.
>>.++.+++++++..<-.>>-
Clean up used cells.
[[-]<]
";

pub const HELLO_WORLD:&str = "
    ++++++++++
 [
  >+++++++>++++++++++>+++>+<<<<-
 ]                       Schleife zur Vorbereitung der Textausgabe
 >++.                    Ausgabe von 'H'
 >+.                     Ausgabe von 'e'
 +++++++.                'l'
 .                       'l'
 +++.                    'o'
 >++.                    Leerzeichen
 <<+++++++++++++++.      'W'
 >.                      'o'
 +++.                    'r'
 ------.                 'l'
 --------.               'd'
 >+.                     '!'
 >.                      Zeilenvorschub
 +++.                    Wagenrücklauf
";
