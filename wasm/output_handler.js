let output_div = document.getElementById("output");

export function clear_output() {
    output_div.textContent = "";
}

export function new_output(output) {
    output_div.textContent += output + "\n";
}

export function revert_output() {
    let output_div = document.getElementById("output");
    let current = output_div.textContent.trimRight();
    output_div.textContent = current.substring(0, current.lastIndexOf("\n") + 1);
}