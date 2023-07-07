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
	if (tabName == "processes") {
		processes();
	}
	if (tabName == "meminfo") {
		meminfo();
	}
	if (tabName == "vmstat") {
		vmStat();
	}
	if (tabName == "kernel_config") {
		kernelConfig(false);
	}
	if (tabName == "sysctl") {
		sysctl(false);
	}
	if (tabName == "interrupts") {
		interrupts();
	}
	if (tabName == "disk_stats") {
		diskStats(false);
	}
	if (tabName == "perfstat") {
		perfStat();
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

// Show landing page
document.getElementById("default").click();
