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
	if (tabName == "cpu_utilization") {
		cpuUtilization();
	}
	if (tabName == "flamegraphs") {
		flamegraphs();
	}
	if (tabName == "top_functions") {
		topFunctions();
	}
	if (tabName == "processes") {
		processes();
	}
	if (tabName == "interrupts") {
		interrupts();
	}
	if (tabName == "perfstat") {
		perfStat();
	}
	if (tabName == "kernel_config") {
		let id = document.querySelector('input[name="kernelDiff"]:checked').id;
		if (id == "kernel_diff_yes") {
			kernelConfig(true);
		} else {
			kernelConfig(false);
		}
	}
	if (tabName == "sysctl") {
		let id = document.querySelector('input[name="sysctlDiff"]:checked').id;
		if (id == "sysctl_diff_yes") {
			sysctl(true);
		} else {
			sysctl(false);
		}
	}
	if (tabName == "meminfo") {
		let id = document.querySelector('input[name="meminfoHide"]:checked').id;
		if (id == "meminfo_hide_yes") {
			meminfo(true);
		} else {
			meminfo(false);
		}
	}
	if (tabName == "vmstat") {
		let id = document.querySelector('input[name="vmstatHide"]:checked').id;
		if (id == "vmstat_hide_yes") {
			vmStat(true);
		} else {
			vmStat(false);
		}
	}
	if (tabName == "disk_stats") {
		let id = document.querySelector('input[name="diskstatHide"]:checked').id;
		if (id == 'diskstat_hide_yes') {
			diskStats(true);
		} else {
			diskStats(false);
		}
	}
	if (tabName == "netstat") {
		let id = document.querySelector('input[name="netstatHide"]:checked').id;
		if (id == "netstat_hide_yes") {
			netStat(true);
		} else {
			netStat(false);
		}
	}
}
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
var elems = document.getElementsByClassName('vmstat-button');
for (var i=0; i < elems.length; i++) {
	elems[i].addEventListener("click",function(evn: Event) {
		if (this.id == "vmstat_hide_yes"){
			vmStat(true);
		} else {
			vmStat(false);
		}
	})
}
var elems = document.getElementsByClassName('diskstat-button');
for (var i=0; i < elems.length; i++) {
	elems[i].addEventListener("click",function(evn: Event) {
		if (this.id == "diskstat_hide_yes"){
			diskStats(true);
		} else {
			diskStats(false);
		}
	})
}
var elems = document.getElementsByClassName('meminfo-button');
for (var i=0; i < elems.length; i++) {
	elems[i].addEventListener("click",function(evn: Event) {
		if (this.id == "meminfo_hide_yes"){
			meminfo(true);
		} else {
			meminfo(false);
		}
	})
}
var elems = document.getElementsByClassName('netstat-button');
for (var i=0; i < elems.length; i++) {
	elems[i].addEventListener("click",function(evn: Event) {
		if (this.id == "netstat_hide_yes"){
			netStat(true);
		} else {
			netStat(false);
		}
	})
}
// Show landing page
document.getElementById("default").click();
