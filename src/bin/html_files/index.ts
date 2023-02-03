import { systemInfo } from './system_info.js';
import { sysctl } from './sysctl.js';
import { cpuUtilization } from './cpu_utilization.js';
import { vmStat } from './vmstat.js';
import { kernelConfig } from './kernel_config.js';
import { interrupts } from './interrupts.js';
import { diskStats } from './disk_stats.js';
import { perfStat } from './perf_stat.js';
export { clearElements, addElemToNode, openData };

function openData(evt: Event, elem: HTMLButtonElement) {
	var tabName: string = elem.name;
	var tabcontent = document.getElementsByClassName('tabcontent');
	var tablinks = document.getElementsByClassName('tablinks');
	for (var i = 0; i < tabcontent.length; i++) {
		(tabcontent[i] as HTMLElement).style.display = "none";
	}
	for (var i = 0; i < tablinks.length; i++) {
		tablinks[i].className = tablinks[i].className.replace(" active", "");
	}
	document.getElementById(tabName).style.display = "block";
	const currentTarget = evt.currentTarget as HTMLButtonElement;
	currentTarget.className += " active";
	if (tabName == "system_info") {
		systemInfo();
	}
	if (tabName == "sysctl") {
		sysctl(false);
	}
	if (tabName == "cpu_utilization") {
		cpuUtilization();
	}
	if (tabName == "vmstat") {
		vmStat();
	}
	if (tabName == "kernel_config") {
		kernelConfig(false);
	}
	if (tabName == "perfstat") {
		perfStat();
	}
	if (tabName == "interrupts") {
		interrupts();
	}
	if (tabName == "disk_stats") {
		diskStats();
	}
}
// Collapse functionality
var coll = Array.from(document.getElementsByClassName("collapsible") as HTMLCollectionOf<HTMLElement>);
coll.forEach((element) => {
	element.addEventListener("click", function () {
		this.classList.toggle("active");
		var content = this.nextElementSibling as HTMLElement;
		if (content.style.display === "block") {
			content.style.display = "none";
		} else {
			content.style.display = "block";
		}
	})
})

// Tab button click
var elems = document.getElementsByClassName('tablinks');
for (var i=0; i < elems.length; i++) {
	elems[i].addEventListener("click", function(evt: Event) {
		openData(evt, this)
	}, false);
}
var elems = document.getElementsByClassName('kernel-button');
for (var i=0; i < elems.length; i++) {
	elems[i].addEventListener("click",function(evn: Event) {
		if (this.id == "kernel_diff_yes"){
			kernelConfig(true);
		} else {
			kernelConfig(false);
		}
	})
}
var elems = document.getElementsByClassName('sysctl-button');
for (var i=0; i < elems.length; i++) {
	elems[i].addEventListener("click",function(evn: Event) {
		if (this.id == "sysctl_diff_yes"){
			sysctl(true);
		} else {
			sysctl(false);
		}
	})
}

function clearElements(id: string) {
	let node: HTMLElement = document.getElementById(id);
	while(node.lastElementChild) {
		node.removeChild(node.lastElementChild);
	}
}

function addElemToNode(node_id: string, elem: HTMLElement) {
	let node: HTMLElement = document.getElementById(node_id);
	node.appendChild(elem);
}

// Show landing page
document.getElementById("default").click();
