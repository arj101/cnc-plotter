#include <Servo.h>
#include <Wire.h>

Servo pen;
volatile int pos = 90;

void setup() {
  Wire.begin(8);                // join i2c bus with address #8
  pen.attach(9);
  // Wire.setClock(100000);
  Wire.onReceive(receiveEvent); // register 
  Serial.begin(9600);           // start serial for output
}

void loop() {
  delay(15);
  if (pos > 180) pos = 180;
  if (pos < 0) pos = 0;
  pen.write(pos);
}

//function that executes whenever data is received from master
//this function is registered as an event, see setup()
void receiveEvent(int howMany) {
  pos = Wire.read();    // receive byte as an integer
}
