$(document).on('click', '[data-toggle="lightbox"]', function(event) {
	event.preventDefault();
	$(this).ekkoLightbox({alwaysShowClose: true});
});

window.addEventListener("load", function() {
	$(document).ready(function() {
		$("#showPassword").change(function() {
			if (this.checked) {
				$('#password').attr('type', 'text');
			} else {
				$('#password').attr('type', 'password');
			}
		});
	});

	window.cookieconsent.initialise({
		"palette": {
			"popup": {
				"background": "#aaa"
			},
			"button": {
				"background": "#f1d600"
			}
		},
		"content": {
			"message": `<div style="font-weight: bold; text-align: center;">Diese Webseite verwendet Cookies</div>

				Wir verwenden Cookies ausschließlich, um die Sicherheit des Logins zu gewährleisten und Ihre Einstellungen zu speichern.
				Sie geben Ihre Einwilligung für die Nutzung von Cookies, wenn Sie unsere Webseite weiterhin nuzten.`,
			"dismiss": "Ich stimme zu",
			"link": "Ausführliche Informationen finden Sie in unserer Datenschutzerklärung",
			"href": "/datenschutz"
		}
	})
});
