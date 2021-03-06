'use strict';
// Some basic runtime stuff needed by a.canvas

// event.repeat is unreliable
// (in particular, it doesn't work on windows)
// So we keep track keydown/keyup for each key to indicate
// whether it's repeating
const downKeys = {};
const DEFAULT_BG_COLOR = '#2A2A2A'
const canvas = document.getElementById('canvas');
const ctx = canvas.getContext('2d');
const canvasbg = document.getElementById('canvasbg');
const ctxbg = canvasbg.getContext('2d');
updateCanvasDim();
let audioPlayed = false;
function newMouseButtonEventListener(type) {
    return function(event) {
        if (!audioPlayed) {
            // playseq(['C4'], 0.5);
            audioPlayed = true;
        }
        external.invoke(
            type +
            '/' +
            event.button +
            '/' +
            event.clientX +
            '/' +
            event.clientY);
        event.preventDefault();
    };
}
function mkke(name, event) {
    var ret = name + '/' + event.key + '/';
    var mods = [];
    if (event.altKey) {
        mods.push('alt');
    }
    if (event.ctrlKey) {
        mods.push('ctrl');
    }
    if (event.metaKey) {
        mods.push('meta');
    }
    if (downKeys[event.key]) {
        // event.repeat doesn't work on Windows
        mods.push('repeat');
    }
    if (event.shiftKey) {
        mods.push('shift');
    }
    return ret + mods.join(',');
}
function updateCanvasDim() {
    canvas.width = canvas.offsetWidth;
    canvas.height = canvas.offsetHeight;
    canvasbg.width = canvasbg.offsetWidth;
    canvasbg.height = canvasbg.offsetHeight;
    ctxbg.fillStyle = DEFAULT_BG_COLOR;
    ctxbg.fillRect(0, 0, canvasbg.width, canvasbg.height);
}
window.addEventListener('resize', function() {
    updateCanvasDim();
    external.invoke('resize/' + canvas.width + '/' + canvas.height);
});
canvas.addEventListener('click', newMouseButtonEventListener('click'));
canvas.addEventListener('mousedown', newMouseButtonEventListener('mousedown'));
canvas.addEventListener('mouseup', newMouseButtonEventListener('mouseup'));
canvas.addEventListener('mousemove', function(event) {
    external.invoke('mousemove/' + event.clientX + '/' + event.clientY);
});
window.addEventListener('keydown', function(event) {
    external.invoke(mkke('keydown', event));
    downKeys[event.key] = true;
    event.preventDefault();
});
window.addEventListener('keyup', function(event) {
    downKeys[event.key] = false;
    external.invoke(mkke('keyup', event));
});
window.addEventListener('keypress', function(event) {
    external.invoke(mkke('keypress', event));
});
external.invoke('init');
function measureText(text) {
    const m = ctx.measureText(text);
    return {
        width: m.width,
        actualBoundingBoxLeft: m.actualBoundingBoxLeft,
        actualBoundingBoxRight: m.actualBoundingBoxRight,
        fontBoundingBoxAscent: m.fontBoundingBoxAscent,
        fontBoundingBoxDescent: m.fontBoundingBoxDescent,
        actualBoundingBoxAscent: m.actualBoundingBoxAscent,
        actualBoundingBoxDescent: m.actualBoundingBoxDescent,
        emHeightAscent: m.emHeightAscent,
        emHeightDescent: m.emHeightDescent,
        hangingBaseline: m.hangingBaseline,
        alphabeticBaseline: m.alphabeticBaseline,
        ideographicBaseline: m.ideographicBaseline,
    };
}
function asyncImageFromBlob(blob) {
    return new Promise(function(resolve, reject) {
        const img = document.createElement('img');
        img.onload = function() {
            resolve(img);
        };
        img.src = URL.createObjectURL(blob);
        external.invoke('debug/' + URL.createObjectURL(blob));
        external.invoke('debug/img.width=' + img.width + ',img.height=' + img.height);
        document.getElementById('misc').appendChild(img);
    });
}
let gamepadData = [];
function handleGamepadEvents() {
    let gamepads = navigator.getGamepads();
    for (var i = 0; i < gamepads.length; i++) {
        let gamepad = gamepads[i];
        if (gamepad === null) {
            continue;
        }
        let index = gamepad.index;
        if (gamepadData.length <= i) {
            gamepadData.push({buttons: [], axes: [], index: index});
        }
        let data = gamepadData[i];
        if (data.index !== index) {
            // If the indices have changed, simply clear out the running data
            // and just rebuild from scratch on the next call to handleGamepadEvents
            gamepadData = [];
            return;
        }
        let buttons = gamepad.buttons;
        for (var j = 0; j < buttons.length; j++) {
            let button = buttons[j];
            if ((typeof button) === 'object') {
                button = button.value;
            }
            if (data.buttons.length <= j) {
                data.buttons.push(0);
            }
            if (data.buttons[j] !== button) {
                external.invoke('gamepadbtn/' + index + '/' + j + '/' + button);
            }
            data.buttons[j] = button;
        }
        let axes = gamepad.axes;
        for (var j = 0; j < axes.length; j++) {
            let axis = axes[j];
            if (data.axes.length <= j) {
                data.axes.push(0);
            }
            if (Math.abs(data.axes[j] - axis) > 0.01) {
                external.invoke('gamepadaxis/' + index + '/' + j + '/' + axis);
            }
            data.axes[j] = axis;
        }
    }
}
window.addEventListener("gamepadconnected", function(event) {
    external.invoke('gamepadconnected/' + event.gamepad.index);
});
var lastTick = null;
function animate(timestamp) {
    requestAnimationFrame(animate);
    if (lastTick === null || timestamp - lastTick > 1000/60) {
        external.invoke('tick/' + timestamp);
        lastTick = timestamp;
    }
    handleGamepadEvents();
}
requestAnimationFrame(animate);
