# Direct Upload

## How to run the application?

Run

```
npm install
npm run dev
```

Open the local website by clicking `b` in the terminal.

## How does the application work?

1. The user selects a file to upload.
2. The user clicks on the upload button.
3. The website sends a request to `/get_session_url`.
4. `/get_session_url` in turn requests authentication through google oauth.
5. After authenticating it sends an request to the google storage api to create a resumable uppload session url.
6. It sends this url as the response from `/get_session_url`.
7. The website stores this url in the local storage till the file has completed uploading.
8. In case on an intruption this url is used to query information about how much of the file was uploaded and to resume the upload.
9. As the upload procceds the update percentage is updated.
