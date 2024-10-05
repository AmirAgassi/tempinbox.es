use yew::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlSelectElement;
use wasm_bindgen::JsCast;
use js_sys;
use wasm_bindgen::closure::Closure;
use reqwest;
use serde::Deserialize;
use std::rc::Rc;
use std::cell::RefCell;
use gloo_timers::callback::Timeout;

#[derive(Deserialize, Clone, Default)]
struct EmailCountResponse {
    code: u32,
    data: u32,
    success: bool,
}

#[derive(Deserialize, Clone, Default)]
struct AvailableEmails {
    allocated: EmailCountResponse,
    hotmail: EmailCountResponse,
    outlook: EmailCountResponse,
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = grecaptcha)]
    fn render(container: &str, params: &JsValue) -> i32;
}

// struct to handle email data
#[derive(Clone, Default)]
struct EmailData {
    email: String,
    password: String,
    refresh_token: String,
    client_id: String,
}

// struct to handle bulk email data
#[derive(Clone, Default)]
struct BulkEmailData {
    email: String,
    password: String,
    refresh_token: String,
    client_id: String,
}

// failed demo counter - might be removed soon
#[derive(Clone)]
struct AnimatedCounter {
    current: Rc<RefCell<f64>>,
    target: u32,
}

impl AnimatedCounter {
    fn new(initial: u32) -> Self {
        AnimatedCounter {
            current: Rc::new(RefCell::new(initial as f64)),
            target: initial,
        }
    }

    fn set_target(&mut self, new_target: u32) {
        self.target = new_target;
    }

    fn animate(&self) {
        let current = self.current.clone();
        let target = self.target as f64;
        
        let f = Rc::new(RefCell::new(None));
        let g = f.clone();

        *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
            let mut current_value = current.borrow_mut();
            let diff = (target - *current_value) * 0.1;
            *current_value += diff;

            if (target - *current_value).abs() < 0.1 {
                *current_value = target;
                let _ = f.borrow_mut().take();
            } else {
                request_animation_frame(f.borrow().as_ref().unwrap());
            }

            // Update the DOM
            if let Some(element) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id(&format!("counter-{}", target)))
            {
                element.set_text_content(Some(&format!("{:.0}", *current_value)));
            }
        }) as Box<dyn FnMut()>));

        request_animation_frame(g.borrow().as_ref().unwrap());
    }
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    web_sys::window()
        .unwrap()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .unwrap();
}

#[function_component(App)]
fn app() -> Html {
    let _email = use_state(|| String::new());
    let loading = use_state(|| false);
    let email_type = use_state(|| "random".to_string());
    let available_emails = use_state(|| AvailableEmails::default());
    let recaptcha_token = use_state(|| String::new());
    let bulk_emails = use_state(|| Vec::<BulkEmailData>::new());
    let bulk_loading = use_state(|| false);
    let email_data = use_state(|| EmailData::default());
    let show_credentials = use_state(|| true);
    let show_bulk_credentials = use_state(|| true);
    let error_message = use_state(|| String::new());

    // Function to fetch and update stock data
    let update_stock_data = {
        let available_emails = available_emails.clone();
        
        Rc::new(RefCell::new(move || {
            let available_emails = available_emails.clone();
            
            spawn_local(async move {
                let response = reqwest::get("https://burneroutlook-api.vercel.app/check_balance")
                    .await
                    .unwrap()
                    .json::<AvailableEmails>()
                    .await
                    .unwrap();
                available_emails.set(response);
            });
        }))
    };

    // Initial fetch of stock data and set up a single delayed fetch
    {
        let update_stock_data = update_stock_data.clone();
        use_effect_with_deps(
            move |_| {
                // Initial fetch
                (update_stock_data.borrow_mut())();

                // Set up a single delayed fetch after 5 seconds
                let delayed_update = update_stock_data.clone();
                let _timeout = Timeout::new(5000, move || {
                    (delayed_update.borrow_mut())();
                });

                // No need for cleanup function as Timeout is dropped automatically
                || ()
            },
            (), // Empty dependencies array
        );
    }

    let onclick = {
        let email_data = email_data.clone();
        let loading = loading.clone();
        let email_type = email_type.clone();
        let recaptcha_token = recaptcha_token.clone();
        Callback::from(move |_| {
            let email_data = email_data.clone();
            let loading = loading.clone();
            let email_type = email_type.clone();
            let recaptcha_token = recaptcha_token.clone();
            loading.set(true);
            spawn_local(async move {
                if recaptcha_token.is_empty() {
                    // show an error message if recaptcha isn't clicked yet
                    web_sys::window()
                        .unwrap()
                        .alert_with_message("Please complete the reCAPTCHA")
                        .unwrap();
                    loading.set(false);
                    return;
                }

                let response = reqwest::get(&format!(
                    "https://burneroutlook-api.vercel.app/get_email?email_type={}&captcha={}",
                    *email_type,
                    *recaptcha_token
                ))
                .await
                .unwrap()
                .text()
                .await
                .unwrap();

                if response.starts_with("Error:") {
                    email_data.set(EmailData {
                        email: response,
                        ..Default::default()
                    });
                } else {
                    let parts: Vec<&str> = response.split(':').collect();
                    if parts.len() == 4 {
                        email_data.set(EmailData {
                            email: parts[0].to_string(),
                            password: parts[1].to_string(),
                            refresh_token: parts[2].to_string(),
                            client_id: parts[3].to_string(),
                        });
                    } else {
                        email_data.set(EmailData {
                            email: "Error: Invalid response format".to_string(),
                            ..Default::default()
                        });
                    }
                }
                loading.set(false);
                
                // reset recaptcha
                let window = web_sys::window().expect("no global `window` exists");
                let grecaptcha = js_sys::Reflect::get(&window, &JsValue::from_str("grecaptcha"))
                    .expect("Failed to get grecaptcha object");
                js_sys::Reflect::get(&grecaptcha, &JsValue::from_str("reset"))
                    .and_then(|reset_fn| {
                        let reset_fn = reset_fn.dyn_into::<js_sys::Function>().unwrap();
                        reset_fn.call0(&JsValue::NULL)
                    })
                    .expect("Failed to reset reCAPTCHA");
                recaptcha_token.set(String::new());
            });
        })
    };

    let bulk_onclick = {
        let bulk_emails = bulk_emails.clone();
        let bulk_loading = bulk_loading.clone();
        let email_type = email_type.clone();
        let recaptcha_token = recaptcha_token.clone();
        let error_message = error_message.clone();
        Callback::from(move |_| {
            let bulk_emails = bulk_emails.clone();
            let bulk_loading = bulk_loading.clone();
            let email_type = email_type.clone();
            let recaptcha_token = recaptcha_token.clone();
            let error_message = error_message.clone();
            bulk_loading.set(true);
            error_message.set(String::new()); // clear any previous error message
            spawn_local(async move {
                if recaptcha_token.is_empty() {
                    web_sys::window()
                        .unwrap()
                        .alert_with_message("Please complete the reCAPTCHA")
                        .unwrap();
                    bulk_loading.set(false);
                    return;
                }

                let response = reqwest::get(&format!(
                    "https://burneroutlook-api.vercel.app/get_bulk_emails?email_type={}&captcha={}",
                    *email_type,
                    *recaptcha_token
                ))
                .await
                .unwrap();

                if response.status() == 429 {
                    error_message.set("Rate limit exceeded. Try again in 24 hours.".to_string());
                    bulk_loading.set(false);
                    return;
                }

                let response_text = response.text().await.unwrap();

                // parse the response into bulkemaildata structs
                let emails: Vec<BulkEmailData> = response_text
                    .lines()
                    .filter_map(|line| {
                        let parts: Vec<&str> = line.split(':').collect();
                        if parts.len() == 4 {
                            Some(BulkEmailData {
                                email: parts[0].to_string(),
                                password: parts[1].to_string(),
                                refresh_token: parts[2].to_string(),
                                client_id: parts[3].to_string(),
                            })
                        } else {
                            None
                        }
                    })
                    .collect();

                bulk_emails.set(emails);
                bulk_loading.set(false);
                
                // reset recaptcha (same as before)
                let window = web_sys::window().expect("no global `window` exists");
                let grecaptcha = js_sys::Reflect::get(&window, &JsValue::from_str("grecaptcha"))
                    .expect("Failed to get grecaptcha object");
                js_sys::Reflect::get(&grecaptcha, &JsValue::from_str("reset"))
                    .and_then(|reset_fn| {
                        let reset_fn = reset_fn.dyn_into::<js_sys::Function>().unwrap();
                        reset_fn.call0(&JsValue::NULL)
                    })
                    .expect("Failed to reset reCAPTCHA");
                recaptcha_token.set(String::new());
            });
        })
    };

    let onchange = {
        let email_type = email_type.clone();
        Callback::from(move |e: Event| {
            let target = e.target_dyn_into::<HtmlSelectElement>();
            if let Some(target) = target {
                email_type.set(target.value());
            }
        })
    };

    use_effect(move || {
        let document = web_sys::window()
            .expect("no global `window` exists")
            .document()
            .expect("should have a document on window");

        // create a callback function
        let recaptcha_token = recaptcha_token.clone();
        let callback = Closure::wrap(Box::new(move || {
            let window = web_sys::window().expect("no global `window` exists");
            let grecaptcha = js_sys::Reflect::get(&window, &JsValue::from_str("grecaptcha"))
                .expect("Failed to get grecaptcha object");

            let render_params = js_sys::Object::new();
            js_sys::Reflect::set(&render_params, &JsValue::from_str("sitekey"), &JsValue::from_str("6LeJlSkqAAAAAMTxxKkrTk32jencF8Go7KqC-6v8")).unwrap();
            js_sys::Reflect::set(&render_params, &JsValue::from_str("theme"), &JsValue::from_str("light")).unwrap();
            
            let token_setter = recaptcha_token.clone();
            js_sys::Reflect::set(&render_params, &JsValue::from_str("callback"), &Closure::wrap(Box::new(move |token: String| {
                token_setter.set(token);
            }) as Box<dyn FnMut(String)>).into_js_value()).unwrap();

            js_sys::Reflect::get(&grecaptcha, &JsValue::from_str("render"))
                .and_then(|render_fn| {
                    let render_fn = render_fn.dyn_into::<js_sys::Function>().unwrap();
                    render_fn.call2(
                        &JsValue::NULL,
                        &JsValue::from_str("recaptcha-container"),
                        &render_params,
                    )
                })
                .expect("Failed to render reCAPTCHA");

            web_sys::console::log_1(&"reCAPTCHA rendered successfully".into());
        }) as Box<dyn FnMut()>);

        // add the recaptcha script
        let script = document.create_element("script").unwrap();
        script.set_attribute("src", "https://www.google.com/recaptcha/api.js?onload=onRecaptchaLoad&render=explicit").unwrap();
        document.head().unwrap().append_child(&script).unwrap();

        // set the callback as a global function
        let window = web_sys::window().expect("no global `window` exists");
        js_sys::Reflect::set(
            &window,
            &JsValue::from_str("onRecaptchaLoad"),
            &callback.as_ref().unchecked_ref(),
        ).expect("Failed to set onRecaptchaLoad");

        // prevent the callback from being dropped
        callback.forget();

        || ()
    });

    let info_expanded = use_state(|| false);
    let toggle_info = {
        let info_expanded = info_expanded.clone();
        Callback::from(move |_| {
            info_expanded.set(!*info_expanded);
        })
    };

    // callback for toggling between credentials and token/id
    let toggle_view = {
        let show_credentials = show_credentials.clone();
        Callback::from(move |_| {
            show_credentials.set(!*show_credentials);
        })
    };

    // callback for toggling between credentials and token/id for bulk emails
    let toggle_bulk_view = {
        let show_bulk_credentials = show_bulk_credentials.clone();
        Callback::from(move |_| {
            show_bulk_credentials.set(!*show_bulk_credentials);
        })
    };

    html! {
        <div class="min-vh-100 d-flex flex-column bg-gradient">
            <main class="flex-grow-1 d-flex align-items-center py-5">
                <div class="container">
                    <div class="row justify-content-center">
                        <div class="col-md-8 col-lg-6">
                            <div class="card shadow-lg border-0 rounded-lg">
                                <div class="card-body p-5">
                                    <div class="text-center">
                                    <img src="https://www.svgrepo.com/show/376347/rust.svg" alt="Rust Logo" style="width: 120px; height: 120px; margin-top: -40px;" />
                                    </div>
                                    <h2 class="card-title text-center fw-bold position-relative" style="margin-top: -20px; z-index: 1;">{"Burner Outlook Generator"}</h2>
                                    <div class="text-center mb-4">
                                        <h5 class="mb-3">{"Stock"}</h5>
                                        <div class="row">
                                            <div class="col-md-4">
                                                <div class="card">
                                                    <div class="card-body">
                                                        <h6 class="card-title">{"Outlook"}</h6>
                                                        <p class="card-text">
                                                            {available_emails.outlook.data}
                                                        </p>
                                                    </div>
                                                </div>
                                            </div>
                                            <div class="col-md-4">
                                                <div class="card">
                                                    <div class="card-body">
                                                        <h6 class="card-title">{"Hotmail"}</h6>
                                                        <p class="card-text">
                                                            {available_emails.hotmail.data}
                                                        </p>
                                                    </div>
                                                </div>
                                            </div>
                                            <div class="col-md-4">
                                                <div class="card">
                                                    <div class="card-body">
                                                        <h6 class="card-title">{"Allocated"}</h6>
                                                        <p class="card-text">
                                                            {available_emails.allocated.data}
                                                        </p>
                                                    </div>
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                    <div class="mb-3">
                                        <label for="email-type" class="form-label">{"Select Email Type"}</label>
                                        <select id="email-type" class="form-select" onchange={onchange}>
                                            <option value="random" selected=true>{"Random"}</option>
                                            <option value="outlook">{"Outlook"}</option>
                                            <option value="hotmail">{"Hotmail"}</option>
                                        </select>
                                    </div>
                                    <div id="recaptcha-container" class="mb-4"></div>
                                    <div class="d-flex justify-content-between mb-4">
                                        <button class="btn btn-primary btn-lg flex-grow-1 me-2" onclick={onclick} disabled={*loading}>
                                            {if *loading { "Generating..." } else { "Generate Email" }}
                                        </button>
                                        <button class="btn btn-primary btn-lg" onclick={bulk_onclick} disabled={*bulk_loading}>
                                            {if *bulk_loading { 
                                                html! { <span class="spinner-border spinner-border-sm" role="status" aria-hidden="true"></span> }
                                            } else { 
                                                html! {
                                                    <>
                                                        <img src="https://www.svgrepo.com/show/24993/lightning-bolt-black-shape.svg" 
                                                             alt="Lightning bolt" 
                                                             style="width: 20px; height: 20px; margin-right: 5px; filter: invert(1);" />
                                                        {"Bulk (10)"}
                                                    </>
                                                }
                                            }}
                                        </button>
                                    </div>
                                    {
                                        if !email_data.email.is_empty() {
                                            html! {
                                                <div class="mt-4">
                                                    <h3 class="h5 mb-3">{"Your Temporary Email"}</h3>
                                                    if email_data.email.starts_with("Error:") {
                                                        <div class="alert alert-danger" role="alert">
                                                            {email_data.email.clone()}
                                                        </div>
                                                    } else {
                                                        <div class="mb-3">
                                                            <button class="btn btn-secondary mb-2" onclick={toggle_view}>
                                                                {if *show_credentials { "Show RefreshToken:ClientID" } else { "Show Username:Password" }}
                                                            </button>
                                                            <div class="input-group">
                                                                <input 
                                                                    type="text" 
                                                                    class="form-control" 
                                                                    value={
                                                                        if *show_credentials {
                                                                            format!("{}:{}", email_data.email, email_data.password)
                                                                        } else {
                                                                            format!("{}:{}", email_data.refresh_token, email_data.client_id)
                                                                        }
                                                                    } 
                                                                    readonly=true 
                                                                />
                                                                <button class="btn btn-outline-secondary" onclick={Callback::from(move |_| {
                                                                    let window = web_sys::window().unwrap();
                                                                    let navigator = window.navigator();
                                                                    let clipboard = navigator.clipboard();
                                                                    let text = if *show_credentials {
                                                                        format!("{}:{}", email_data.email, email_data.password)
                                                                    } else {
                                                                        format!("{}:{}", email_data.refresh_token, email_data.client_id)
                                                                    };
                                                                    let _ = clipboard.write_text(&text);
                                                                })}>
                                                                    {"Copy"}
                                                                </button>
                                                            </div>
                                                        </div>
                                                    }
                                                </div>
                                            }
                                        } else {
                                            html! {}
                                        }
                                    }
                                    {
                                        if !error_message.is_empty() {
                                            html! {
                                                <div class="alert alert-danger mt-4" role="alert">
                                                    {(*error_message).clone()}
                                                </div>
                                            }
                                        } else {
                                            html! {}
                                        }
                                    }
                                    {
                                        if !bulk_emails.is_empty() {
                                            html! {
                                                <div class="mt-4">
                                                    <h3 class="h5 mb-3">{"Your Bulk Temporary Emails"}</h3>
                                                    <button class="btn btn-secondary mb-2" onclick={toggle_bulk_view}>
                                                        {if *show_bulk_credentials { "Show RefreshToken:ClientID" } else { "Show Username:Password" }}
                                                    </button>
                                                    <ul class="list-group">
                                                        {bulk_emails.iter().map(|email_data| {
                                                            html! {
                                                                <li class="list-group-item">
                                                                    {if *show_bulk_credentials {
                                                                        format!("{}:{}", email_data.email, email_data.password)
                                                                    } else {
                                                                        format!("{}:{}", email_data.refresh_token, email_data.client_id)
                                                                    }}
                                                                </li>
                                                            }
                                                        }).collect::<Html>()}
                                                    </ul>
                                                    <button class="btn btn-outline-secondary mt-2" onclick={Callback::from(move |_| {
                                                        let window = web_sys::window().unwrap();
                                                        let navigator = window.navigator();
                                                        let clipboard = navigator.clipboard();
                                                        let text = bulk_emails.iter().map(|email_data| {
                                                            if *show_bulk_credentials {
                                                                format!("{}:{}", email_data.email, email_data.password)
                                                            } else {
                                                                format!("{}:{}", email_data.refresh_token, email_data.client_id)
                                                            }
                                                        }).collect::<Vec<String>>().join("\n");
                                                        let _ = clipboard.write_text(&text);
                                                    })}>
                                                        {"Copy All"}
                                                    </button>
                                                </div>
                                            }
                                        } else {
                                            html! {}
                                        }
                                    }
                                </div>
                                <div class="card-footer text-center">
                                    <button 
                                        class="btn btn-link text-muted" 
                                        onclick={toggle_info}
                                    >
                                        {if *info_expanded { "Hide Technical Details" } else { "How Does It Work?" }}
                                    </button>
                                    {
                                        if *info_expanded {
                                            html! {
                                                <div class="mt-4 text-start">
                                                    <h4 class="mb-3">{"How It Works: Key Features"}</h4>
                                                    <p>{"This system uses some awesome techniques to generate temporary email addresses, including:"}</p>
                                                    <ul>
                                                        <li>{"Custom FunCaptcha bypass, with 2000 raw recognitions/day provided graciously by "}<a href="https://nopecha.com/pricing" target="_blank" rel="noopener noreferrer">{"NopeCHA!"}</a></li>
                                                        <li>{"Rotating residential proxies"}</li>
                                                        <li>{"Low-level API interactions for Microsoft Auth/Funcaptcha - this stuff's "}<strong>{"HARD!"}</strong></li>
                                                    </ul>
                                                    <p>{"This project mainly serves as a practical exercise in maintaining a long-term, complex reverse engineering software project. As long as the service is still running more than a few months from it's creation, I consider the challenge a success."}</p>
                                                    <p>{"Note: If you see zero allocated accounts, don't worry! I'm definitely aware of it and actively working on patching any new changes. I take a lot of pride in keeping this service running and will be sure to have it up and running by the next day."}</p>
                                                </div>
                                            }
                                        } else {
                                            html! {}
                                        }
                                    }
                                </div>
                                <div class="card-footer text-center text-muted">
                                    <small>
                                        {"For more cool projects, check out my "}
                                        <a href="https://github.com/AmirAgassi" target="_blank" rel="noopener noreferrer" class="text-muted">
                                            {"GitHub"}
                                        </a>
                                        {"!"}
                                    </small>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </main>
        </div>
    }
}

fn main() {
    yew::start_app::<App>();
}