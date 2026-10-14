export const demoFiles = [
 { path:'forge.rs',content:`pub struct Grimoire {
    pages: Vec<String>,
    ink: u32,
}
impl Grimoire {
    pub fn transmute(&mut self, source: String) {
        let words = source.split_whitespace();
        let mut power = 0;
        for word in words {
            if word.len() > 3 {
                power += 1;
            } else {
                power += 2;
            }
        }
        self.inscribe(power);
        self.seal();
        return;
    }
    fn inscribe(&self, power: u32) {
        illuminate(power);
    }
    fn seal(&self) {
        illuminate(1);
    }
}
fn illuminate(power: u32) {
    let rune = power * 2;
    for step in 0..rune {
        let light = step + power;
        if light > 4 {
            emit(light);
        }
    }
    emit(rune);
}`},
 {path:'moon.py',content:`class Moon:
    def awaken(self, phase):
        light = 0
        for star in range(phase):
            if star > 2:
                light += star
            else:
                light += 1
        self.reflect(light)
        return light

    def reflect(self, light):
        return light * 2
`},
 {path:'prism.ts',content:`interface Spell {
    name: string;
    power: number;
}
function weave(spell: Spell) {
    let light = spell.power;
    const runes = [];
    for (let i = 0; i < light; i++) {
        if (i > 2) {
            runes.push(i);
        } else {
            light += 1;
        }
    }
    sparkle(runes);
    return runes;
}
function sparkle(runes: number[]) {
    return runes.length;
}`},
 {path:'sun.js',content:`const altar = { flame: 3, crystal: true };
function radiate(power) {
    let light = power;
    for (let ray = 0; ray < 8; ray++) {
        if (ray % 2 === 0) {
            light += ray;
        }
    }
    return light;
}`},
 {path:'river.go',content:`type River struct {
    Depth int
}
func ripple(depth int) {
    for i := 0; i < depth; i++ {
        if i > 2 {
            flow(i)
        }
    }
    flow(depth)
}
func flow(depth int) {
    println(depth)
}`},
 {path:'Clock.c',content:`struct Clock { int hour; int minute; };
void tick(int hour) {
    for (int i = 0; i < hour; i++) {
        if (i > 2) {
            chime(i);
        }
    }
    chime(hour);
}
void chime(int hour) { return; }`},
 {path:'Temple.java',content:`class Temple {
    public void summon(int power) {
        int light = power;
        for (int i = 0; i < power; i++) {
            light += i;
        }
        bless(light);
    }
    private void bless(int light) { return; }
}`},
 {path:'Crystal.cs',content:`class Crystal {
    public void Refract(int power) {
        int light = power;
        for (int i = 0; i < power; i++) {
            light += i;
        }
        Shine(light);
    }
    private void Shine(int light) { return; }
}`},
 {path:'gears.cpp',content:`class Gear {
    void spin(int power) {
        for (int i = 0; i < power; i++) {
            if (i > 2) {
                turn(i);
            }
        }
        turn(power);
    }
    void turn(int step) { return; }
};`}
];
