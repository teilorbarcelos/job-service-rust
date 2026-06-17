pub fn get_audit_explorer_view() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Audit & Error Explorer | Admin</title>
    <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;600;700&display=swap" rel="stylesheet">
    <style>
        :root {
            --primary: #6366f1;
            --primary-hover: #4f46e5;
            --bg: #0f172a;
            --card-bg: rgba(30, 41, 59, 0.7);
            --text: #f8fafc;
            --text-dim: #94a3b8;
            --border: rgba(255, 255, 255, 0.1);
            --danger: #ef4444;
            --success: #22c55e;
            --warning: #f59e0b;
        }

        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
            font-family: 'Outfit', sans-serif;
        }

        body {
            background-color: var(--bg);
            color: var(--text);
            min-height: 100vh;
            background-image: 
                radial-gradient(at 0% 0%, rgba(99, 102, 241, 0.15) 0, transparent 50%),
                radial-gradient(at 100% 100%, rgba(239, 68, 68, 0.1) 0, transparent 50%);
            display: flex;
            flex-direction: column;
        }

        header {
            padding: 2rem;
            display: flex;
            justify-content: space-between;
            align-items: center;
            border-bottom: 1px solid var(--border);
            backdrop-filter: blur(10px);
            position: sticky;
            top: 0;
            z-index: 100;
        }

        .logo {
            font-size: 1.5rem;
            font-weight: 700;
            background: linear-gradient(to right, #818cf8, #f472b6);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }

        main {
            padding: 2rem;
            flex: 1;
            max-width: 1400px;
            margin: 0 auto;
            width: 100%;
        }

        .controls {
            display: flex;
            gap: 1rem;
            margin-bottom: 2rem;
            flex-wrap: wrap;
            align-items: center;
        }

        .tabs {
            display: flex;
            gap: 0.5rem;
            background: var(--card-bg);
            padding: 0.4rem;
            border-radius: 12px;
            border: 1px solid var(--border);
        }

        .tab {
            padding: 0.6rem 1.2rem;
            border-radius: 8px;
            cursor: pointer;
            transition: all 0.3s ease;
            font-weight: 600;
            font-size: 0.9rem;
            color: var(--text-dim);
            border: none;
            background: transparent;
        }

        .tab.active {
            background: var(--primary);
            color: white;
            box-shadow: 0 4px 12px rgba(99, 102, 241, 0.3);
        }

        .search-box {
            flex: 1;
            min-width: 300px;
            position: relative;
        }

        input {
            width: 100%;
            background: var(--card-bg);
            border: 1px solid var(--border);
            padding: 0.8rem 1rem;
            border-radius: 12px;
            color: white;
            outline: none;
            transition: border-color 0.3s ease;
        }

        input:focus {
            border-color: var(--primary);
        }

        .btn {
            padding: 0.8rem 1.5rem;
            border-radius: 12px;
            cursor: pointer;
            font-weight: 600;
            transition: all 0.3s ease;
            display: flex;
            align-items: center;
            gap: 0.5rem;
            border: none;
        }

        .btn-primary {
            background: var(--primary);
            color: white;
        }

        .btn-primary:hover {
            background: var(--primary-hover);
            transform: translateY(-2px);
        }

        .login-overlay {
            position: fixed;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            background: rgba(15, 23, 42, 0.95);
            backdrop-filter: blur(10px);
            display: flex;
            justify-content: center;
            align-items: center;
            z-index: 2000;
        }

        .login-card {
            background: var(--card-bg);
            padding: 2.5rem;
            border-radius: 20px;
            border: 1px solid var(--border);
            width: 90%;
            max-width: 400px;
            box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
            display: flex;
            flex-direction: column;
            gap: 1.5rem;
        }

        .login-header {
            text-align: center;
            margin-bottom: 0.5rem;
        }

        .login-title {
            font-size: 1.5rem;
            font-weight: 700;
            margin-bottom: 0.5rem;
            background: linear-gradient(to right, #818cf8, #f472b6);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }

        .form-group {
            display: flex;
            flex-direction: column;
            gap: 0.5rem;
        }

        .form-label {
            font-size: 0.9rem;
            color: var(--text-dim);
            font-weight: 600;
            text-align: left;
        }

        .form-input {
            width: 100%;
            background: rgba(0, 0, 0, 0.2);
            border: 1px solid var(--border);
            padding: 0.8rem 1rem;
            border-radius: 12px;
            color: white;
            outline: none;
            transition: border-color 0.3s ease;
        }

        .form-input:focus {
            border-color: var(--primary);
        }

        .btn-full {
            width: 100%;
            justify-content: center;
            margin-top: 1rem;
        }

        .login-error {
            color: var(--danger);
            font-size: 0.85rem;
            text-align: center;
            min-height: 20px;
        }

        .table-container {
            background: var(--card-bg);
            border-radius: 16px;
            border: 1px solid var(--border);
            overflow: hidden;
            backdrop-filter: blur(10px);
            box-shadow: 0 20px 50px rgba(0, 0, 0, 0.3);
        }

        table {
            width: 100%;
            border-collapse: collapse;
            text-align: left;
            table-layout: fixed;
        }

        th {
            padding: 1.2rem;
            background: rgba(255, 255, 255, 0.03);
            font-weight: 600;
            font-size: 0.85rem;
            text-transform: uppercase;
            letter-spacing: 0.05em;
            color: var(--text-dim);
            border-bottom: 1px solid var(--border);
        }

        td {
            padding: 1rem 1.2rem;
            border-bottom: 1px solid var(--border);
            font-size: 0.9rem;
            vertical-align: middle;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }

        tr:hover td {
            background: rgba(255, 255, 255, 0.02);
        }

        .badge {
            padding: 0.3rem 0.6rem;
            border-radius: 6px;
            font-size: 0.75rem;
            font-weight: 700;
            text-transform: uppercase;
        }

        .badge-get { background: rgba(34, 197, 94, 0.2); color: #4ade80; }
        .badge-post { background: rgba(99, 102, 241, 0.2); color: #818cf8; }
        .badge-put { background: rgba(245, 158, 11, 0.2); color: #fbbf24; }
        .badge-delete { background: rgba(239, 68, 68, 0.2); color: #f87171; }
        .badge-patch { background: rgba(168, 85, 247, 0.2); color: #c084fc; }

        .pagination {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-top: 1.5rem;
            padding: 0 0.5rem;
        }

        .page-info {
            color: var(--text-dim);
            font-size: 0.9rem;
        }

        .page-controls {
            display: flex;
            gap: 0.5rem;
        }

        .page-btn {
            width: 40px;
            height: 40px;
            display: flex;
            align-items: center;
            justify-content: center;
            border-radius: 10px;
            background: var(--card-bg);
            border: 1px solid var(--border);
            cursor: pointer;
            color: var(--text);
            transition: all 0.2s ease;
        }

        .page-btn:hover:not(:disabled) {
            border-color: var(--primary);
            background: rgba(99, 102, 241, 0.1);
        }

        .page-btn:disabled {
            opacity: 0.3;
            cursor: not-allowed;
        }

        .json-preview {
            color: var(--primary);
            font-weight: 600;
            cursor: pointer;
            text-decoration: underline;
        }

        .modal-overlay {
            position: fixed;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            background: rgba(0, 0, 0, 0.8);
            backdrop-filter: blur(5px);
            display: none;
            justify-content: center;
            align-items: center;
            z-index: 1000;
        }

        .modal {
            background: var(--bg);
            width: 90%;
            max-width: 800px;
            max-height: 80vh;
            border-radius: 20px;
            border: 1px solid var(--border);
            display: flex;
            flex-direction: column;
            overflow: hidden;
        }

        .modal-header {
            padding: 1.5rem;
            border-bottom: 1px solid var(--border);
            display: flex;
            justify-content: space-between;
            align-items: center;
        }

        .modal-body {
            padding: 1.5rem;
            overflow-y: auto;
            background: #000;
            font-family: 'Fira Code', monospace;
            font-size: 0.9rem;
            color: #4ade80;
            white-space: pre-wrap;
        }

        .close-modal {
            cursor: pointer;
            font-size: 1.5rem;
            color: var(--text-dim);
        }

        .loading {
            position: absolute;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            background: var(--card-bg);
            display: flex;
            justify-content: center;
            align-items: center;
            z-index: 50;
            border-radius: 16px;
        }

        .spinner {
            width: 40px;
            height: 40px;
            border: 4px solid rgba(255, 255, 255, 0.1);
            border-top-color: var(--primary);
            border-radius: 50%;
            animation: spin 1s linear infinite;
        }

        @keyframes spin { to { transform: rotate(360deg); } }

        .hidden { display: none !important; }
    </style>
</head>
<body>
    <header>
        <div class="logo">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"></path></svg>
            Audit Explorer
        </div>
        <div class="user-info" id="userDisplayName">
            Admin Mode
        </div>
    </header>

    <main>
        <div class="controls">
            <div class="tabs">
                <button class="tab active" data-type="audit">Audit Logs</button>
                <button class="tab" data-type="errors">Error Logs</button>
            </div>
            
            <div class="search-box">
                <input type="text" id="searchInput" placeholder="Search by user, table, or action...">
            </div>

            <button class="btn btn-primary" id="searchBtn">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" 
                    stroke-linecap="round" stroke-linejoin="round">
                    <circle cx="11" cy="11" r="8"></circle>
                    <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
                </svg>
                Search
            </button>
        </div>

        <div class="table-container" style="position: relative; min-height: 400px;">
            <div id="loader" class="loading hidden">
                <div class="spinner"></div>
            </div>
            <table id="logsTable">
                <thead>
                    <tr id="tableHeader">
                        <!-- Dynamic Header -->
                    </tr>
                </thead>
                <tbody id="tableBody">
                    <!-- Dynamic Body -->
                </tbody>
            </table>
        </div>

        <div class="pagination">
            <div class="page-info" id="pageInfo">
                Showing page 1 of 1
            </div>
            <div class="page-controls">
                <button class="page-btn" id="prevBtn" disabled>
                    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6"></polyline></svg>
                </button>
                <button class="page-btn" id="nextBtn" disabled>
                    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"></polyline></svg>
                </button>
            </div>
        </div>
    </main>

    <div class="modal-overlay" id="modalOverlay">
        <div class="modal">
            <div class="modal-header">
                <h3 id="modalTitle">Details</h3>
                <span class="close-modal" id="closeModal">&times;</span>
            </div>
            <div class="modal-body" id="modalContent"></div>
        </div>
    </div>

    <div class="login-overlay hidden" id="loginOverlay">
        <div class="login-card">
            <div class="login-header">
                <h2 class="login-title">Audit Explorer</h2>
                <p style="color: var(--text-dim); font-size: 0.9rem;">Sign in to access logs</p>
            </div>
            
            <form id="loginForm">
                <div class="form-group">
                    <label class="form-label">Email</label>
                    <input type="email" id="loginEmail" class="form-input" required placeholder="admin@example.com">
                </div>
                
                <div class="form-group" style="margin-top: 1rem;">
                    <label class="form-label">Password</label>
                    <input type="password" id="loginPassword" class="form-input" required placeholder="••••••••">
                </div>

                <div id="loginErrorMsg" class="login-error"></div>

                <button type="submit" class="btn btn-primary btn-full" id="loginSubmitBtn">
                    Sign In
                </button>
            </form>
        </div>
    </div>

    <script>
        let currentType = 'audit';
        let currentPage = 0;
        let totalPages = 1;
        const pageSize = 15;
        let currentItems = [];

        const tableHeader = document.getElementById('tableHeader');
        const tableBody = document.getElementById('tableBody');
        const pageInfo = document.getElementById('pageInfo');
        const loader = document.getElementById('loader');
        const modalOverlay = document.getElementById('modalOverlay');
        const modalContent = document.getElementById('modalContent');
        const modalTitle = document.getElementById('modalTitle');

        const headers = {
            audit: ['Date', 'User', 'Action', 'Method', 'Table', 'URL', 'Details'],
            errors: ['Date', 'User', 'Source', 'Message', 'Details']
        };

        const fetchLogs = async () => {
            loader.classList.remove('hidden');
            const search = document.getElementById('searchInput').value;
            const url = `/admin/api/${currentType}?page=${currentPage}&size=${pageSize}&search=${search}`;
            
            try {
                const res = await fetch(url, {
                    headers: { 'Authorization': 'Bearer ' + localStorage.getItem('token') }
                });
                
                if (res.status === 401 || res.status === 403) {
                    showLogin();
                    return;
                }

                const data = await res.json();
                
                if (!res.ok) {
                    throw new Error(data.message || 'Server error fetching logs');
                }

                currentItems = data.items || [];
                renderTable(currentItems);
                totalPages = Math.ceil((data.total || 0) / pageSize);
                updatePagination();
            } catch (err) {
                console.error('[AuditExplorer] Fetch logs error:', err);
                alert('Error fetching logs: ' + err.message);
            } finally {
                loader.classList.add('hidden');
            }
        };

        const renderTable = (items) => {
            tableHeader.innerHTML = '';
            headers[currentType].forEach(h => {
                const th = document.createElement('th');
                th.textContent = h;
                tableHeader.appendChild(th);
            });
            
            tableBody.innerHTML = '';
            if (!items || items.length === 0) {
                tableBody.innerHTML = `<tr><td colspan="${headers[currentType].length}" style="text-align: center; padding: 3rem; color: var(--text-dim);">No logs found</td></tr>`;
                return;
            }

            items.forEach((item, i) => {
                const tr = document.createElement('tr');
                if (currentType === 'audit') {
                    tr.innerHTML = `
                        <td>${new Date(item.created_at).toLocaleString()}</td>
                        <td title="${item.user_name}">${item.user_name}</td>
                        <td>${item.action_type}</td>
                        <td><span class="badge badge-${item.method?.toLowerCase()}">${item.method}</span></td>
                        <td>${item.table_name}</td>
                        <td title="${item.base_url}">${item.base_url || ''}</td>
                        <td><span class="json-preview view-details" data-index="${i}">View Details</span></td>
                    `;
                } else {
                    tr.innerHTML = `
                        <td>${new Date(item.created_at).toLocaleString()}</td>
                        <td>${item.id_user || 'System'}</td>
                        <td title="${item.source}">${item.source || ''}</td>
                        <td style="color: var(--danger)" title="${item.error_message}">${item.error_message || ''}</td>
                        <td><span class="json-preview view-details" data-index="${i}">View Error</span></td>
                    `;
                }
                tableBody.appendChild(tr);
            });
        };

        const updatePagination = () => {
            pageInfo.textContent = `Showing page ${currentPage + 1} of ${totalPages || 1}`;
            document.getElementById('prevBtn').disabled = currentPage === 0;
            document.getElementById('nextBtn').disabled = currentPage >= totalPages - 1;
        };

        const viewDetails = (index) => {
            const item = currentItems[index];
            modalTitle.textContent = currentType === 'audit' ? 'Audit Record Details' : 'Error Log Details';
            modalContent.textContent = JSON.stringify(item, null, 4);
            modalOverlay.style.display = 'flex';
        };

        document.addEventListener('click', (e) => {
            if (e.target.classList.contains('view-details')) {
                const index = e.target.getAttribute('data-index');
                viewDetails(index);
            }
        });

        const loginOverlay = document.getElementById('loginOverlay');
        const loginErrorMsg = document.getElementById('loginErrorMsg');
        const loginSubmitBtn = document.getElementById('loginSubmitBtn');
        const loginForm = document.getElementById('loginForm');

        const showLogin = () => {
            loginOverlay.classList.remove('hidden');
            localStorage.removeItem('token');
        };

        const hideLogin = () => {
            loginOverlay.classList.add('hidden');
            loginErrorMsg.textContent = '';
        };

        const handleLogin = async (e) => {
            e.preventDefault();
            const email = document.getElementById('loginEmail').value;
            const password = document.getElementById('loginPassword').value;
            
            loginSubmitBtn.disabled = true;
            loginSubmitBtn.textContent = 'Signing in...';
            loginErrorMsg.textContent = '';

            try {
                const loginUrl = window.location.origin + '/v1/auth/login';

                const res = await fetch(loginUrl, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ email, password })
                });

                const data = await res.json();

                if (!res.ok) {
                    throw new Error(data.message || 'Invalid credentials');
                }

                const token = data.token || data.accessToken || data.access_token;
                
                if (token) {
                    localStorage.setItem('token', token);
                    hideLogin();
                    fetchLogs();
                } else {
                    console.error('[AuditExplorer] Login failed: No token in response');
                    throw new Error('Token not received');
                }
            } catch (err) {
                console.error('[AuditExplorer] Login error:', err);
                loginErrorMsg.textContent = err.message;
            } finally {
                loginSubmitBtn.disabled = false;
                loginSubmitBtn.textContent = 'Sign In';
            }
        };

        if (loginForm) {
            loginForm.addEventListener('submit', handleLogin);
        }

        document.querySelectorAll('.tab').forEach(tab => {
            tab.addEventListener('click', () => {
                document.querySelector('.tab.active').classList.remove('active');
                tab.classList.add('active');
                currentType = tab.dataset.type;
                currentPage = 0;
                fetchLogs();
            });
        });

        document.getElementById('searchBtn').addEventListener('click', () => {
            currentPage = 0;
            fetchLogs();
        });

        document.getElementById('searchInput').addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                currentPage = 0;
                fetchLogs();
            }
        });

        document.getElementById('prevBtn').addEventListener('click', () => {
            if (currentPage > 0) {
                currentPage--;
                fetchLogs();
            }
        });

        document.getElementById('nextBtn').addEventListener('click', () => {
            if (currentPage < totalPages - 1) {
                currentPage++;
                fetchLogs();
            }
        });

        document.getElementById('closeModal').addEventListener('click', () => modalOverlay.style.display = 'none');
        window.onclick = (e) => { if (e.target === modalOverlay) modalOverlay.style.display = 'none'; };

        if (!localStorage.getItem('token')) {
            showLogin();
        } else {
            fetchLogs();
        }
    </script>
</body>
</html>
"#
}
